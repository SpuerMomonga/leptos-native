# Event Loop and Async Runtime

The event loop is owned by Tao. The async runtime is Tokio. The reactive scheduler is `any_spawner` configured with a Tao-aware executor. All three live in `crates/leptos-native`.

## Single Event Loop

There is exactly one Tao event loop per process. It owns:

- Native window events (input, focus, lifecycle).
- Tray-icon events forwarded as user events.
- Custom user events sent via `EventLoopProxy`.
- Reactive executor wakeups (`UserEvent::PollLocal`).
- WebView IPC events (`UserEvent::WebViewEvent`, `UserEvent::WebViewControl`) — under the `webview` feature.

This holds for both backends. The Blitz backend does not introduce a winit loop; it uses the same Tao loop and asks Blitz to render into wgpu surfaces created from Tao window handles.

## User Event Type

Defined in `crates/leptos-native::event_loop`:

```rust
pub enum UserEvent {
    PollLocal,
    Dispatch(Box<dyn FnOnce(&App) + Send>),
    Tray(tray_icon::TrayIconEvent),
    Menu(tray_icon::menu::MenuEvent),
    WindowOpen(Window),
    WindowOpenFailed(WindowId, WindowError),
    WindowClose(WindowId),
    Shutdown,

    #[cfg(feature = "webview")]
    WebViewEvent(WindowId, leptos_native_ipc::IpcEvent),
    #[cfg(feature = "webview")]
    WebViewControl(WindowId, leptos_native_ipc::Control),
}
```

The user-event channel is the only way other threads talk to the main thread. Tray and menu events are forwarded via `tray-icon`'s registration callbacks at startup.

## Tokio Integration

Pattern follows dioxus-desktop:

1. `Application::run` builds a Tokio multi-thread `Runtime`.
2. The runtime hosts a `LocalSet` bound to the main thread.
3. `tokio::spawn` runs on the worker pool for `Send` work.
4. `App::spawn_local` (and the reactive executor) push to the `LocalSet` for non-`Send` work — effects, view-tree state, renderer mutations.

The Tao loop runs on the main thread. A `PollLocal` user event polls the local set once. A waker injected into the local set sends `PollLocal` whenever a task becomes ready. This keeps async tasks alive without hijacking the main thread and guarantees rendering work runs in lock-step with native events.

## The Tao-Aware Executor

`crates/leptos-native::executor::init_for_tao(proxy)` calls `any_spawner::Executor::init_custom(spawn, spawn_local, poll_local)`:

- `spawn(future)` → `tokio::spawn(future)`.
- `spawn_local(future)` → push to a thread-local `LocalSet` queue and `proxy.send_event(UserEvent::PollLocal)`.
- `poll_local()` → poll the queue once.

When a signal updates and an effect needs to run, the call goes through `spawn_local`, which posts `PollLocal`. Tao receives the event, polls the local set, the effect runs, and renderer mutations apply on the main thread.

## Frame Loop

The two backends differ on what a "frame" means:

- **`webview-dom`** — there is no in-process frame; the WebView paints itself. After a Tao tick, the framework calls `WebViewBackend::flush_mutations` to ship any pending batch.
- **`blitz-dom`** — the framework requests redraws via `Window::request_redraw`. Tao delivers `WindowEvent::RedrawRequested`; Blitz computes layout if dirty, paints with Vello, and presents through wgpu. Reactive mutations mark the document dirty, which schedules a redraw.

The reactive executor does not gate updates on a frame. It runs effects eagerly; each effect mutates renderer state immediately. For Blitz, that mutation flips the document's dirty flag; for WebView, it appends to an outbound batch.

## Per-Tick Order

Inside one Tao tick:

1. Tao delivers a window or user event.
2. Input events translate through the active backend and dispatch to listeners; user handlers update signals.
3. Signal updates schedule effect polls.
4. The local set polls; effects run; renderer mutations apply.
5. For Blitz, dirty windows request redraw. For WebView, the outbound batch flushes.
6. Tao moves on.

This ordering avoids torn states: by the time a frame paints, all reactive consequences of the input event have applied.

## Threading Rules

- All view tree state, all signals owned by views, and all effects live on the main thread.
- `tokio::spawn` is for I/O and CPU work that does not touch the view tree directly. Such tasks deliver results back to the main thread by setting signals or sending custom user events.
- `Renderer` impls (`WebViewRenderer`, `BlitzDomRenderer`) are `Send + 'static` but per-window backend state is not required to be `Send`. The framework keeps it on the main thread.

## Shutdown

`UserEvent::Shutdown` causes the loop to exit:

1. Drop every `Window`. This cancels their root effects and tears down backend resources (`WebViewBackend::detach` or `BlitzBackend::detach`).
2. Drop the `Application`'s root owner.
3. Stop the Tokio runtime (`Runtime::shutdown_timeout`).
4. Tao loop exits.

Tray-icon registrations drop with the `Application`.

## Why Not Two Loops

A common alternative is to run Tao on the main thread and Tokio on its own thread, communicating through channels. The framework rejects that because:

- Reactive effects must run on the main thread to drive renderer mutations safely.
- Crossing threads for every effect adds latency and forces `Send` bounds onto user code.
- Tao's `EventLoopProxy::send_event` already provides cross-thread wakeups.

The single-loop design matches dioxus-desktop and produces the simplest mental model: native events, async tasks, and reactive effects all flow through one timeline.
