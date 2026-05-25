# Windowing

Multi-window support is a core capability. The window API lives in `crates/leptos-native::window`.

## Window Model

A `Window` is the framework's per-window unit. It owns:

- A Tao `tao::window::Window`.
- A backend instance — a `webview_dom::WebViewBackend` or a `blitz_dom::BlitzBackend`, depending on the active feature.
- A root `Owner` that scopes signals and effects for this window.
- A mounted view subtree built from the user-supplied component.
- Local context values registered through `provide_context`.

`Window` is itself the runtime handle. It is `Clone + Send`; clones share an `Arc` of the inner state. There is no separate `WindowHandle` type — operating on a `Window` value directly *is* the API. Pass it into background tasks, store it in components, or hand it back to the `App` registry; they are all the same handle.

Each window's tree is independent. State sharing between windows happens through `Application`-scoped contexts, stores, or channels.

## Identifying Windows

Two identifiers:

- `WindowId(String)` — user-supplied logical name (`"main"`, `"tools"`). Stable, unique within the app.
- Internal Tao `WindowId` — issued by Tao for event routing. The framework maps Tao's `WindowId` to its own so user code never sees the Tao type.

`App::get_window(id)` looks up a window by logical name; `App::get_windows()` enumerates all open windows.

## `WindowBuilder`

Full surface in [README.md](README.md). Recap:

```rust
let window = WindowBuilder::new("main", Counter)
    .title("counter")
    .inner_size(640, 480)
    .resizable(true)
    .build();
```

`Window::builder("id", Component)` is a shorthand alias for `WindowBuilder::new("id", Component)`. Both forms appear in user code.

`WindowBuilder::build` returns a configured-but-not-opened `Window`. Hand it to `App::open_window(window)` to register it with the runtime — that is what triggers the Tao + backend bring-up.

## HTML Shell

Both backends render HTML/CSS, so `WindowBuilder::html_shell` provides a common starting document. Default:

- Empty `<body>` with one `<div id="root">` mount point.
- Reset stylesheet appropriate for desktop apps.

A user can supply a custom shell to inject fonts, theme variables, or a layout grid. The framework writes the WebView IPC bridge or the Blitz event hooks on top of the shell at mount time.

## Lifecycle

1. `WindowBuilder::build` returns a `Window` — configured but not yet opened. The Tao window does not exist yet; no GPU surface is allocated; no backend is attached.
2. `App::open_window(window)` registers the `Window` with the runtime. The runtime stores the handle in its registry keyed by `WindowId`, then schedules bring-up on the next tick:
   - Builds the Tao `Window` from `WindowConfig`.
   - Constructs the backend instance via `WebViewBackend::attach` or `BlitzBackend::attach`.
   - Creates the window's root `Owner`.
   - Runs the user's component function inside that owner.
   - Mounts the resulting view subtree against the backend's `renderer_root`.
3. While running, the window forwards Tao events to its backend, dispatches DOM-style events to listeners, and runs reactive effects.
4. `window.close()` (or `App::close_window(&id)`, or the user clicking the OS close control) posts `UserEvent::WindowClose(id)` to the Tao loop. The loop:
   - Runs the optional `on_close` handler. If it returns `CloseAction::Cancel`, the close is vetoed and nothing else happens.
   - Drops the mounted view (effects cancel, listeners detach).
   - Detaches the backend (`WebViewBackend::detach` or `BlitzBackend::detach`).
   - Removes the entry from `App`'s registry.
   - Drops the Tao window.

Closing the last window does not implicitly exit the application unless configured to. Tray-only apps keep the loop alive without windows. Default exit policy: "exit when the last window closes."

Existing `Window` clones held outside the registry stay valid in shape (the `Arc` is still alive), but operations on them become no-ops once the registry has dropped its entry. `window.is_closed()` reports the current state.

## Programmatic Window Operations

Per-window operations live on `Window` itself:

```rust
impl Window {
    pub fn close(&self);
    pub fn focus(&self);
    pub fn show(&self);
    pub fn hide(&self);
    pub fn minimize(&self);
    pub fn maximize(&self);
    pub fn set_title(&self, title: impl Into<String>);
    pub fn set_inner_size(&self, w: u32, h: u32);
    pub fn set_position(&self, x: i32, y: i32);

    pub fn is_visible(&self) -> bool;
    pub fn is_focused(&self) -> bool;
    pub fn is_closed(&self) -> bool;
    pub fn inner_size(&self) -> (u32, u32);
}
```

These dispatch through the Tao event loop, so they are safe from any thread that holds a `Window` clone (cross-thread calls go through `UserEvent::Dispatch`).

`App` only manages the registry:

```rust
impl App {
    pub fn open_window(&self, window: Window);
    pub fn close_window(&self, id: &WindowId);
    pub fn get_window(&self, id: &WindowId) -> Option<Window>;
    pub fn get_windows(&self) -> Vec<Window>;
}
```

Closing through `App::close_window(&id)` and `window.close()` end at the same code path; pick whichever the call site already has in scope.

Components receive their enclosing `Window` via context:

```rust
let window = use_window();
window.close();
window.minimize();
```

`use_window()` reads the `Window` provided by the framework at mount time.

## Per-Window Context

- `provide_context(value)` adds a value to the current owner's scope, propagating to all descendants in this window.
- Cross-window values come from `App::provide_context` and are visible inside every window.
- Each window has its own `Window` value in its scope (provided automatically), retrievable via `use_window()`.

## Backend Attachment

Both backends consume the Tao window's raw handle:

- **`webview-dom`** — `wry::WebViewBuilder::new_as_child(&tao_window)`.
- **`blitz-dom`** — creates a wgpu `Surface` from `RawWindowHandle`/`RawDisplayHandle`, builds a Vello renderer, drives paint on `RedrawRequested`.

The user does not see this difference. Both arrive as a `Window` with the same API.

## Errors

`WindowError` lists failure modes:

- `BackendInit(String)` — Wry or Blitz failed to attach to the Tao window.
- `MissingComponent` — builder built without a component (only reachable through `try_build` on a manually mutated builder).
- `InvalidGeometry` — min > max, etc.
- `Os(String)` — underlying OS error from Tao.

`build` panics on misconfiguration. `try_build` returns the error.

## Why Window-Is-Handle

Splitting `Window` into a configured-state value and a separate `WindowHandle` runtime type means users hold two different things to mean "this window". It also makes the `App::open_window(window)` boundary unclear: do you pass the configured value or the runtime handle? Collapsing both into one `Clone + Send` `Window` removes the distinction. Before `open_window` it represents intent; after, it represents the live window. The same methods work on both sides — pre-open mutators can either fail or be queued for replay; the spec is "queue and replay" so calls on a not-yet-open window apply as soon as it opens.

## Why Not Multi-Process Per Window

A multi-process model (one renderer process per window) is out of scope. Both backends run in-process. Windows share the Tokio runtime, the reactive scheduler, and the user-event channel. This is the simplest and most performant shape for a desktop framework; multi-process can be revisited if a real isolation requirement appears.
