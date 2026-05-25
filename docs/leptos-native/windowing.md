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

A `WindowBuilder` is bound to an `App` handle and finalized by a fallible `build`. This mirrors `tauri::WebviewWindowBuilder`: configure fluently, then call `build` to synchronously create the Tao window, attach the active backend, mount the component, and register the window with the runtime.

```rust
let window = WindowBuilder::new(app, "main", Counter)
    .title("counter")
    .inner_size(640, 480)
    .resizable(true)
    .build()?;
```

`Window::builder(app, "id", Component)` is a shorthand alias for `WindowBuilder::new(app, "id", Component)`. Both forms appear in user code.

`build` returns `leptos_native::Result<Window>`. On `Ok`, the window is already live, registered in `App`, and visible (unless `.visible(false)` was set). On `Err`, nothing was registered and no native resources remain. There is no separate "configured-but-not-opened" state and no asynchronous open step. The unified error model is documented in [errors.md](../errors.md).

## HTML Shell

Both backends render HTML/CSS, so `WindowBuilder::html_shell` provides a common starting document. Default:

- Empty `<body>` with one `<div id="root">` mount point.
- Reset stylesheet appropriate for desktop apps.

A user can supply a custom shell to inject fonts, theme variables, or a layout grid. The framework writes the WebView IPC bridge or the Blitz event hooks on top of the shell at mount time.

## Lifecycle

1. `WindowBuilder::build` runs synchronously on the calling thread and:
   - Builds the Tao `Window` from `WindowConfig`.
   - Constructs the backend instance via `WebViewBackend::attach` or `BlitzBackend::attach`.
   - Creates the window's root `Owner`.
   - Runs the user's component function inside that owner.
   - Mounts the resulting view subtree against the backend's `renderer_root`.
   - Registers the `Window` with the runtime keyed by `WindowId`.
   - Returns the live `Window`.

   Any failure in these steps returns `Err(leptos_native::Error)` and leaves nothing registered.
2. While running, the window forwards Tao events to its backend, dispatches DOM-style events to listeners, and runs reactive effects.
3. `window.close()` (or `App::close_window(&id)`, or the user clicking the OS close control) posts `UserEvent::WindowClose(id)` to the Tao loop. The loop:
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
    pub fn close_window(&self, id: &WindowId);
    pub fn get_window(&self, id: &WindowId) -> Option<Window>;
    pub fn get_windows(&self) -> Vec<Window>;
}
```

Window creation does not go through `App` — `WindowBuilder::build` registers the window directly. Closing through `App::close_window(&id)` and `window.close()` end at the same code path; pick whichever the call site already has in scope.

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

`WindowBuilder::build` returns `leptos_native::Result<Window>` (`leptos_native::Error` is an alias for `anyhow::Error`; the unified error model lives in [errors.md](../errors.md)). All failure modes surface synchronously through this single call, matching `tauri::WebviewWindowBuilder::build`. Common causes — each carrying a chained context message and, where useful, a downcastable sentinel from `leptos_native::error::kind`:

- Backend attach failure — Wry or Blitz failed to attach to the Tao window. Downcasts to `kind::WebViewMissing` or `kind::GpuAdapterMissing` when the cause is identifiable.
- `kind::InvalidGeometry` — min > max, negative size, etc. Caught at `build` time.
- `kind::DuplicateWindow(WindowId)` — a window with this logical id is already registered.
- OS errors from Tao when creating the native window — propagated as anonymous causes inside the chain.
- Missing component — surfaced as a panic, not a `Result`. Building a `WindowBuilder` without a component is a programmer error (see the panic policy in [errors.md](../errors.md)).

Apps handle window-creation errors at the call site, the same way they would handle any other fallible setup step:

```rust
use leptos_native::error::kind;

Application::default()
    .setup(|app| {
        let main = WindowBuilder::new(app, "main", Counter)
            .title("leptos-native")
            .inner_size(960, 640)
            .build();

        match main {
            Ok(_) => {}
            Err(err) if err.is::<kind::WebViewMissing>() => {
                show_install_webview_dialog();
                app.quit();
            }
            Err(err) => {
                tracing::error!(?err, "main window failed to open");
                app.quit();
            }
        }
    })
    .run();
```

There is no asynchronous error channel for window construction. A window either exists when `build` returns `Ok`, or the call returned `Err` and nothing was registered.

## Why Window-Is-Handle

Splitting `Window` into a configured-state value and a separate `WindowHandle` runtime type means users hold two different things to mean "this window". Collapsing both into one `Clone + Send` `Window` removes the distinction. The same methods work on every clone — pass a `Window` into a background task, store it in a component, or hand it back through `App::get_window`; they are all the same handle. Because `WindowBuilder::build` produces a live window directly, there is no pre-open phase to model separately.

## Why Not Multi-Process Per Window

A multi-process model (one renderer process per window) is out of scope. Both backends run in-process. Windows share the Tokio runtime, the reactive scheduler, and the user-event channel. This is the simplest and most performant shape for a desktop framework; multi-process can be revisited if a real isolation requirement appears.
