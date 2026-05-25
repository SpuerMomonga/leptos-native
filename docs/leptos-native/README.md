# `crates/leptos-native` (umbrella)

The `leptos-native` crate is the public entry point. It owns the Tao event loop, the Tokio runtime, the Tao-aware `any_spawner` executor, the window registry, the optional system tray, and the `Application`/`App`/`Window`/`WindowBuilder` API. It selects exactly one DOM backend (`webview-dom` or `blitz-dom`) at compile time via mutually exclusive features.

`Window` is itself the runtime handle. There is no separate `WindowHandle` type: `Window` is `Clone + Send`, internally `Arc`-shared, and its methods (`close`, `focus`, `show`, `set_title`, …) route through the Tao user-event channel.

This is the only crate application code typically imports.

## Responsibility

- Build and own the Tao `EventLoop`.
- Build and park the Tokio runtime.
- Initialize the Tao-aware `any_spawner::Executor` so reactive effects schedule onto the main thread.
- Hold the window registry; instantiate the active backend per window.
- Drive the system tray when configured.
- Run the user-supplied `setup` closure with an `App` handle.
- Drive the event loop until shutdown.

`leptos-native` coordinates subsystems but does not absorb their internal logic. Window mechanics, backend internals, IPC, and the renderer trait live elsewhere.

## Dependencies

- `crates/render` — re-exports types via `prelude`.
- `crates/view-macro` — re-exports `view!` and `#[component]` via `prelude`.
- One of `crates/webview-dom` / `crates/blitz-dom` — selected via features.
- `tao` — event loop and windowing.
- `tokio` — async runtime.
- `any_spawner` — executor.
- `tray-icon` — system tray.
- `reactive_graph` (re-export through `prelude`).

## Features

```toml
[features]
default = ["webview"]
webview = ["dep:leptos-native-webview-dom"]
blitz   = ["dep:leptos-native-blitz-dom"]
```

`webview` and `blitz` are **mutually exclusive**. Building with both enabled fails at compile time with a clear `compile_error!`. Building with **neither** enabled also fails with a clear `compile_error!` — there is no headless default. Pick one explicitly:

```toml
# pick one
leptos-native = { version = "0.1", features = ["webview"] }
leptos-native = { version = "0.1", default-features = false, features = ["blitz"] }
```

When the `default-features = false` form is used to switch off `webview`, the `blitz` feature must be added in the same line.

## Public API

```rust
pub mod prelude {
    pub use crate::{Application, App, Window, WindowBuilder, Tray, TrayBuilder, MenuItem, TrayMenu};
    pub use crate::backend::Renderer;            // The active backend's Renderer impl.
    pub use leptos_native_render::{Render, Mountable, IntoView, ListenerHandle};
    pub use leptos_native_view_macro::{view, component};
    pub use reactive_graph::signal::*;
    pub use reactive_graph::computed::Memo;
    pub use reactive_graph::effect::Effect;
    pub use reactive_graph::owner::Owner;
    pub use reactive_graph::traits::*;
}
```

`backend::Renderer` is a type alias resolving to `leptos_native_webview_dom::WebViewRenderer` or `leptos_native_blitz_dom::BlitzDomRenderer` depending on the active feature. Application code can write `R = backend::Renderer` if it ever needs to spell the renderer; ordinary `view!` code never does.

### `Application`

```rust
pub struct Application { /* ... */ }

impl Application {
    pub fn new() -> Self;
    pub fn default() -> Self;

    pub fn name(self, name: impl Into<String>) -> Self;
    pub fn version(self, version: impl Into<String>) -> Self;
    pub fn id(self, identifier: impl Into<String>) -> Self;

    pub fn setup<F>(self, setup: F) -> Self where F: FnOnce(&App) + 'static;

    pub fn run(self);
    pub fn run_with(self, on_event: impl FnMut(&App, AppEvent) + 'static);
}

/// Application-level events surfaced to `run_with`.
pub enum AppEvent {
    /// The Tao loop has started; setup is complete and windows are open.
    Started,
    /// A window finished bring-up successfully.
    WindowOpened(WindowId),
    /// A window's bring-up failed (GPU adapter missing, system WebView not installed,
    /// HTML shell rejected by Blitz, etc). Surfaces backend errors that cannot be
    /// returned synchronously from `App::open_window`.
    WindowOpenFailed(WindowId, WindowError),
    /// A window finished tearing down. Fires after `on_close` and registry removal.
    WindowClosed(WindowId),
    /// The loop is about to exit. Last chance for synchronous cleanup before drop.
    WillExit,
}
```

`Application::default()` produces a working app with sensible defaults: a Tao loop, a multi-thread Tokio runtime, the Tao-aware executor.

`name` and `version` are optional. When unset, they fall back to the consuming crate's `CARGO_PKG_NAME` and `CARGO_PKG_VERSION` (read at compile time via `env!`), so a typical app never needs to call them. Override only when the displayed app name should differ from the Cargo package name (e.g., `"my_app_bin"` → `"My App"`) or when the runtime version should not track Cargo's.

`id` has no default and must be set explicitly. It is the reverse-DNS application identifier (e.g., `"com.example.counter"`) used by platform APIs that need a stable, globally-unique handle:

- Windows: `SetCurrentProcessExplicitAppUserModelID`, which controls taskbar grouping, jump lists, and toast notification attribution. Must be called before the first window is shown.
- Linux: D-Bus object path segment for `libayatana-appindicator` tray integration.
- Single-instance lock key (named mutex on Windows, abstract socket on Linux), if/when single-instance support is added.

These cannot be derived from `CARGO_PKG_*` because the Cargo package name is not reverse-DNS-formatted and there is no way to encode an organization domain there. Forcing `id` keeps platform integration correct.

### `App`

```rust
pub struct App { /* ... */ }

impl App {
    // Window registry
    pub fn open_window(&self, window: Window);
    pub fn close_window(&self, id: &WindowId);
    pub fn get_window(&self, id: &WindowId) -> Option<Window>;
    pub fn get_windows(&self) -> Vec<Window>;

    // Tray
    pub fn set_tray(&self, tray: Tray);

    // Cross-window context
    pub fn provide_context<T: Send + Sync + 'static>(&self, value: T);

    // Lifecycle
    pub fn quit(&self);

    // Async
    pub fn spawn<F>(&self, fut: F) -> tokio::task::JoinHandle<F::Output>
    where F: Future + Send + 'static, F::Output: Send + 'static;
    pub fn spawn_local<F>(&self, fut: F) -> tokio::task::JoinHandle<F::Output>
    where F: Future + 'static;
}
```

`App` only manages the *registry* — opening, closing, and looking up windows. Per-window operations (`focus`, `show`, `hide`, `set_title`, `close`, geometry queries) live on `Window` itself; see [windowing.md](windowing.md) for the full surface.

`open_window` takes a built `Window`, not a `WindowBuilder`. Constructing the builder and finalizing with `.build()` happens on the caller's side; `open_window` is the boundary that hands the configured window to the runtime.

`close_window(&id)` removes the window from the registry and triggers its teardown. The same effect can be reached from inside the window via `window.close()` — both routes go through the Tao user-event channel and end at the same registry-removal path.

`App` is `Clone` and `Send`. Operations that must run on the main thread route through the user-event channel internally.

### `Window` and `WindowBuilder`

```rust
#[derive(Clone)]
pub struct Window { /* Arc-shared inner state */ }

impl Window {
    pub fn builder<C, V>(id: impl Into<WindowId>, component: C) -> WindowBuilder
    where C: FnOnce() -> V + 'static, V: IntoView;

    // Identity
    pub fn id(&self) -> &WindowId;

    // Lifecycle
    pub fn close(&self);
    pub fn focus(&self);
    pub fn show(&self);
    pub fn hide(&self);
    pub fn minimize(&self);
    pub fn maximize(&self);

    // Mutators
    pub fn set_title(&self, title: impl Into<String>);
    pub fn set_inner_size(&self, w: u32, h: u32);
    pub fn set_position(&self, x: i32, y: i32);

    // Queries
    pub fn is_visible(&self) -> bool;
    pub fn is_focused(&self) -> bool;
    pub fn inner_size(&self) -> (u32, u32);
}
```

`Window` is the runtime handle. `Clone` is cheap (it shares an `Arc` of inner state) so a window reference can travel with components, into background tasks, or back into `App` lookups. There is no separate `WindowHandle` type — operating on a `Window` value directly is the API.

`window.close()` posts a `UserEvent::WindowClose(id)` to the Tao loop, which removes the entry from `App`'s registry and tears down the backend, the root `Owner`, and the Tao window in that order. Calling `close` on an already-closed `Window` is a no-op.

`WindowBuilder` configures a window before it is opened:

```rust
pub struct WindowBuilder { /* ... */ }

impl WindowBuilder {
    pub fn new<C, V>(id: impl Into<WindowId>, component: C) -> Self
    where C: FnOnce() -> V + 'static, V: IntoView;

    pub fn title(self, title: impl Into<String>) -> Self;
    pub fn inner_size(self, w: u32, h: u32) -> Self;
    pub fn min_inner_size(self, w: u32, h: u32) -> Self;
    pub fn max_inner_size(self, w: u32, h: u32) -> Self;
    pub fn position(self, x: i32, y: i32) -> Self;
    pub fn resizable(self, on: bool) -> Self;
    pub fn decorations(self, on: bool) -> Self;
    pub fn always_on_top(self, on: bool) -> Self;
    pub fn transparent(self, on: bool) -> Self;
    pub fn maximized(self, on: bool) -> Self;
    pub fn fullscreen(self, mode: FullscreenMode) -> Self;
    pub fn visible(self, on: bool) -> Self;
    pub fn icon(self, icon: Icon) -> Self;
    pub fn html_shell(self, shell: HtmlShell) -> Self;
    pub fn stylesheet(self, css: impl Into<String>) -> Self;
    pub fn on_close(self, handler: impl FnMut(&Window) -> CloseAction + 'static) -> Self;

    pub fn build(self) -> Window;
    pub fn try_build(self) -> Result<Window, WindowError>;
}
```

`WindowBuilder::new(id, component)` and `Window::builder(id, component)` are aliases. `build` panics on misconfiguration; `try_build` returns the error. The returned `Window` is configured but not opened — calling `App::open_window(window)` registers it with the runtime, which performs the Tao + backend bring-up on the first event-loop tick.

The full `Window` lifecycle is documented in [windowing.md](windowing.md).

### `Tray` and `TrayBuilder`

See [tray.md](tray.md) for the full surface. The umbrella re-exports the public types.

## Internal Layout

```
crates/leptos-native/
  src/
    lib.rs              -- prelude, public re-exports
    application.rs      -- Application, App
    window.rs           -- Window, WindowBuilder
    tray.rs             -- Tray, TrayBuilder, MenuItem
    event_loop.rs       -- UserEvent, dispatch loop
    executor.rs         -- Tao-aware any_spawner executor
    runtime.rs          -- Tokio runtime + LocalSet plumbing
    backend/
      mod.rs            -- selects WebViewBackend or BlitzBackend by feature
      webview.rs        -- thin wrapper around webview-dom (under cfg(feature = "webview"))
      blitz.rs          -- thin wrapper around blitz-dom (under cfg(feature = "blitz"))
```

The `backend` module is the only place feature-gated `cfg` lives in this crate. Public types stay backend-neutral.

## Lifecycle

1. `Application::run`:
   - Builds the Tokio multi-thread runtime.
   - Builds the Tao `EventLoop` and stores its proxy.
   - Calls `executor::init_for_tao(proxy)` (delegates to `any_spawner::Executor::init_custom`).
   - Registers the static `tray-icon` event handlers, forwarding to user events.
   - Calls the user `setup` closure with an `App` handle.
   - Opens every registered window: builds the Tao `Window`, attaches the active backend, mounts the user component, mounts the view tree.
   - Builds the tray if one was set.
   - Enters the Tao loop.
2. While running, the loop dispatches Tao events, user events (tray, menu, IPC events from the WebView backend, executor wakeups), reactive polls, and per-tick mutation flush + redraw scheduling.
3. On `App::quit` or last-window-close-with-default-policy, owners drop, backends detach, the runtime shuts down, the loop exits.

## Why a Single Umbrella

- One stable public surface for application code.
- One place to wire Tao + Tokio + executor + tray + backend.
- Backend swapping is a feature flag — application code does not change.
- Future subsystems (state stores, plugin systems) can grow inside this crate until they have a real reuse boundary, then split out.

## What This Crate Does Not Own

- The `Renderer` trait — `crates/render`.
- The view tree and reactive bridge — `crates/render`.
- The `view!` and `#[component]` macros — `crates/view-macro`.
- IPC types — `crates/ipc`.
- Backend internals — `crates/webview-dom` or `crates/blitz-dom`.
- Developer tooling — `crates/cli`.
