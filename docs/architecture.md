# Architecture

`leptos-native` is a Rust-native desktop framework that pairs Leptos's reactive primitives with a swappable rendering backend. All user code is plain native Rust — no WebAssembly, no browser API surface.

## Goals

- All application code runs as native Rust, compiled for the host platform.
- Two rendering backends, selectable at compile time:
  - **WebView** (`webview-dom`): Wry hosts an HTML/CSS shell; Rust drives DOM mutations through IPC.
  - **Blitz** (`blitz-dom`): Blitz paints HTML/CSS natively via Vello/wgpu; no WebView involved.
- Multi-window applications, system tray integration, and async work share a single Tao event loop.
- The same component code works against both backends without rewrites.

## Layered Model

```
+----------------------------------------------------------------+
|  application code (components, signals, view! macros)          |
+----------------------------------------------------------------+
|  view-macro          (view! and #[component])                  |
+----------------------------------------------------------------+
|  render              (Renderer trait, Render<R>, view tree)    |
+----------------------------------------------------------------+
|  upstream reactive_graph + any_spawner                         |
+----------------------------------------------------------------+
|  ipc          (typed mutation/event messages, webview only)    |
+----------------------------------------------------------------+
|  webview-dom (Wry + JS interpreter)   blitz-dom (Vello + wgpu) |
+----------------------------------------------------------------+
|  leptos-native       (Application, Window, tray, event loop)   |
+----------------------------------------------------------------+
|  Tao + Tokio + tray-icon + OS                                  |
+----------------------------------------------------------------+
```

The `Renderer` trait sits at the load-bearing seam. Everything above it is backend-agnostic; everything below is backend-specific.

## Reuse vs. Reimplement

Reused unchanged from upstream Leptos:

- `reactive_graph` — fully DOM-agnostic. Signals, memos, effects, owners, batching.
- `any_spawner` — pluggable executor; `init_custom` lets us drive effects from the Tao loop.
- `reactive_stores` — optional, for store-based state.

Reimplemented in this workspace:

- `crates/render` — own `Renderer` trait and view tree. Upstream `tachys` 0.7+ hardcoded `Rndr = web_sys::Dom` and removed the renderer generic; it cannot host a non-DOM backend without forking. Pachys hit the same wall and forked. We write a narrower replacement.
- `crates/view-macro` — own `view!` and `#[component]` so the macros emit `crates/render` types instead of `tachys` DOM types.

## Core Idea: One View Tree, Two Backends

The pattern (proven by pachys): define a `Renderer` trait with associated `Node`/`Element`/`Text`/`Placeholder` types and tree-mutation methods. Implement it once per backend. Reactive closures (`FnMut() -> impl Render<R>`) become `RenderEffect`s that re-run on signal changes and call the renderer's mutation methods. The view layer never names a backend.

## Backend Comparison

| Concern | webview-dom (Wry) | blitz-dom (Blitz) |
|---|---|---|
| Rendering | HTML/CSS in a system WebView | Native paint via Vello/wgpu |
| Layout | Browser engine | Taffy + Parley (built into Blitz) |
| Distribution | Smaller binary, depends on system WebView2/WKWebView | Larger binary, no system WebView dependency |
| Dev experience | DevTools, full CSS spec | No browser DevTools; pure Rust stack |
| `Element` type | `WebViewElementId(u64)` mirrored on the JS side | `blitz_dom::NodeId` |
| Mutation transport | IPC into a JS interpreter | Direct method calls on `blitz_dom` |
| Event transport | IPC from JS handler | Blitz event dispatch |

Application code is identical. Switch by changing the `leptos-native` feature flag.

## Crate Map

See [repository.md](repository.md) for full layout. The shape:

- `crates/render` — `Renderer` trait, `Render<R>`, `Mountable<R>`, view tree types. Depends on upstream `reactive_graph`.
- `crates/view-macro` — `view!` and `#[component]` macros emitting `crates/render` types.
- `crates/ipc` — typed messages for the WebView backend's mutation/event protocol.
- `crates/webview-dom` — Wry-based `Renderer` impl plus the embedded JS interpreter. Depends on `render`, `ipc`, `wry`, `tao`.
- `crates/blitz-dom` — Blitz-based `Renderer` impl plus the Vello paint pipeline. Depends on `render`, `blitz-dom` upstream, `vello`, `wgpu`, `tao`.
- `crates/leptos-native` — umbrella entry crate. Owns `Application`, `App`, `Window`, tray, Tao event loop, Tokio runtime, the Tao-aware `any_spawner` executor. Selects one DOM crate via mutually exclusive features (`webview` / `blitz`).
- `crates/cli` — developer tool: scaffolding, asset pipeline, run helpers, packaging.

## Design Constraints

- The `Renderer` trait is the only seam between view code and backends. Application code never names a backend type.
- The two DOM crates are real, separate crates because the `Renderer` boundary is stable and reused across projects (matches dioxus-desktop / dioxus-native and pachys).
- Exactly one of `webview` / `blitz` is enabled per `leptos-native` build. Features are mutually exclusive.
- Tao owns the event loop unconditionally. The Blitz backend integrates by wrapping Tao windows; it does not pull in winit.
- All IPC payloads are typed Rust structs. No string-typed control channels.
- The reactive graph is the only state model. Effects are scheduled onto the Tao main thread through a custom `any_spawner` executor wired in `crates/leptos-native`.
- `unsafe` is forbidden in new crates without an explicit, documented need.

## Dependency Direction

```
reactive_graph (upstream)  ─┐
                            ├─> render ─> view-macro
                            │
                            ├─> ipc ─> webview-dom ─┐
                            │                       ├─> leptos-native
                            └─> blitz-dom ──────────┘
```

`cli` depends on workspace metadata and build helpers, not on runtime crates.

`webview-dom` and `blitz-dom` are alternatives; `leptos-native` depends on whichever its active feature selects. Both DOM crates depend on `render` and on `tao` (for window handle integration).

## Lifecycle (Single Window)

1. `Application::default()` builds a Tao `EventLoop`, a Tokio runtime, and initializes the Tao-aware `any_spawner` executor.
2. `setup` callback runs; user constructs windows via `WindowBuilder`.
3. Each window's component function returns a view tree built from the active backend's renderer types.
4. The window mounts the view tree, the backend attaches to the Tao window (Wry WebView or Blitz wgpu surface), reactive effects start running.
5. Event loop runs. Native events (input, lifecycle, tray) flow through Tao; renderer events (clicks, key presses) flow through backend-specific channels and end as signal updates.
6. On shutdown, owners drop, effects cancel, backends tear down, the loop exits.

## What This Architecture Does Not Try To Do

- Target the browser. The WASM build of Leptos is a separate path; this framework is desktop-only.
- Implement a virtual DOM. Mutations are pushed directly through the renderer trait, driven by fine-grained reactivity.
- Abstract the Tao/Wry/Blitz/Tokio stack behind generic platform names. Names stay plain.
- Provide cross-platform mobile support. Scope is desktop (Windows, macOS, Linux).

## Tracked As Future Work (Not Yet Designed)

These are real concerns the framework will need answers for, but they are not in scope for the v0 design:

- **Accessibility (a11y)**. `webview-dom` inherits the system WebView's accessibility tree for free; `blitz-dom` currently has no a11y story. Closing this gap means integrating `accesskit` (or equivalent) at the Blitz layer. Apps that ship today on `blitz-dom` will not be screen-reader compatible.
- **Input Method Editors (IME)**. Same shape: the WebView handles CJK / Korean / complex script input out of the box; Blitz needs explicit Tao IME-event plumbing into Parley. Deferred.
- **HiDPI / scaling**. `webview-dom` follows the system. `blitz-dom` needs explicit scale-factor propagation from Tao to the wgpu surface and the Blitz viewport. Will be addressed alongside multi-monitor support.
- **Hot reload**. `view!` macro output is structured to allow a future hot-reload plugin (à la `leptos_hot_reload` / `dioxus-hot-reload`), but no work is done up front.
- **Single-instance lock**. The `Application::id` is the future key; the lock itself is not implemented yet.

These are listed here so that no one assumes "the framework supports X" by reading the architecture and finding silence on the topic. If you build an app that needs any of them, treat the corresponding backend as not-yet-suitable.
