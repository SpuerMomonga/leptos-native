# Tech Stack

Organized by technology first, then by which crate each technology lands in.

## Tao (windowing + event loop)

Used by both backends and by `leptos-native`.

- `EventLoop`, `EventLoopBuilder`, `EventLoopProxy`, `ControlFlow`.
- `WindowBuilder`, `Window`, `WindowEvent`.

Lives in: `crates/leptos-native` (loop ownership), `crates/webview-dom` and `crates/blitz-dom` (window-handle integration).

## Wry (WebView backend)

- `WebViewBuilder`, `WebView`.
- `with_html`, `with_initialization_script`, `with_ipc_handler`, `with_asynchronous_custom_protocol`.
- `evaluate_script`.
- `build(&window)`.

Lives in: `crates/webview-dom`.

## Blitz (Blitz backend)

- `blitz_dom::Document`, `NodeId`.
- Element/text/attribute mutation methods on `Document`.
- `blitz_html::HtmlDocument` for shell bootstrap.
- A Blitz renderer crate (e.g., `blitz_renderer_vello`) for paint.
- `blitz_traits::Viewport` and event types.

Lives in: `crates/blitz-dom`.

## wgpu + Vello (Blitz backend)

- `wgpu::Instance`, `Surface`, `Device`, `Queue`.
- `vello::Renderer`, `vello::Scene` (driven by Blitz's renderer crate).
- `raw_window_handle` traits.

Lives in: `crates/blitz-dom`.

## tray-icon (system tray)

- `TrayIconBuilder`, `TrayIcon`, `TrayIconEvent`.
- `menu::Menu`, `menu::MenuItem`, `menu::IconMenuItem`, `menu::PredefinedMenuItem`, `menu::MenuEvent`.

Lives in: `crates/leptos-native`.

## Leptos Reactive Primitives (upstream, reused)

`reactive_graph` is DOM-agnostic and pulled in directly:

- `reactive_graph::signal::*` (`signal`, `RwSignal`, `ReadSignal`, `WriteSignal`).
- `reactive_graph::computed::Memo`.
- `reactive_graph::effect::Effect`, `RenderEffect`.
- `reactive_graph::owner::Owner`.
- `reactive_graph::traits::*` (`Get`, `Set`, `Update`, `With`, `Track`).

`any_spawner::Executor::init_custom` plugs the Tao-aware executor.

Optional: `reactive_stores` for store-based state.

Lives in: `crates/render` (depends on `reactive_graph`), `crates/leptos-native` (initializes `any_spawner`).

The framework does **not** depend on upstream `tachys`, `leptos_dom`, or `leptos_macro`. Those hardcode the DOM renderer in 0.7+; `crates/render` and `crates/view-macro` replace them.

## Tokio (async runtime)

- `runtime::Builder::new_multi_thread`, `Runtime`.
- `tokio::spawn`, `spawn_blocking`.
- `LocalSet` for non-`Send` task coordination on the main thread.

Lives in: `crates/leptos-native`.

## serde + serde_json (IPC)

Used by the WebView backend.

Lives in: `crates/ipc`, `crates/webview-dom`.

## Crate-to-Tech Map

| Crate | Wraps |
|---|---|
| `crates/render` | `reactive_graph` (only) |
| `crates/view-macro` | `proc-macro2`, `syn`, `quote` |
| `crates/ipc` | `serde`, `serde_json` |
| `crates/webview-dom` | `wry`, `tao`, `serde`, `serde_json`; depends on `ipc` and `render` |
| `crates/blitz-dom` | `blitz-dom` (upstream), `vello`, `wgpu`, `tao`; depends on `render` |
| `crates/leptos-native` | `tao`, `tokio`, `any_spawner`, `tray-icon`; depends on one of `webview-dom` / `blitz-dom` via features |
| `crates/cli` | `clap`, build-time helpers |

## Design Constraints

- Keep windowing, tray, and backend names explicit in module paths.
- Two backends, exactly one selected per binary at compile time.
- Reactive scheduling runs on the Tao main thread; never block it.
- Reuse upstream Leptos crates wherever they work without wrapping (`reactive_graph`, `any_spawner`).
- Reimplement only what upstream Leptos forces (the renderer-generic view layer).
