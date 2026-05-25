# Docs

Design notes, architecture, and protocol specifications for `leptos-native`.

## Top-Level

- [architecture.md](architecture.md) — layered architecture, dependency direction, design constraints.
- [repository.md](repository.md) — workspace layout and crate boundaries.
- [tech-stack.md](tech-stack.md) — third-party technologies and how they map to the framework.

## Per-Crate

Each crate has its own folder under `docs/`. Open the folder's `README.md` for the crate's design surface; subsystem files (when present) live alongside it.

- [render/](render/README.md) — `Renderer` trait, `Render<R>`, view tree types, the reactive bridge built on upstream `reactive_graph`.
- [view-macro/](view-macro/README.md) — `view!` and `#[component]` macros.
- [ipc/](ipc/README.md) — typed mutation and event protocol used by the WebView backend.
- [webview-dom/](webview-dom/README.md) — Wry-based renderer crate.
- [blitz-dom/](blitz-dom/README.md) — Blitz-based renderer crate.
- [leptos-native/](leptos-native/README.md) — umbrella crate: `Application`, `App`, `Window`, tray, Tao event loop, Tokio runtime, executor wiring.
  - [leptos-native/event-loop.md](leptos-native/event-loop.md) — Tao ownership, Tokio integration, reactive executor.
  - [leptos-native/windowing.md](leptos-native/windowing.md) — multi-window model and lifecycle.
  - [leptos-native/tray.md](leptos-native/tray.md) — system tray and menu design.
- [cli/](cli/README.md) — developer CLI surface.

## Cross-Cutting

- [rendering.md](rendering.md) — backend selection and comparison.
- [examples.md](examples.md) — example conventions.

## Reading Order

1. Start with [architecture.md](architecture.md) for the big picture.
2. Read [repository.md](repository.md) and [tech-stack.md](tech-stack.md) for the workspace and dependency map.
3. Read [render/](render/README.md) and [view-macro/](view-macro/README.md) to see how user code reaches a renderer.
4. Pick a backend in [rendering.md](rendering.md), then dive into [webview-dom/](webview-dom/README.md) or [blitz-dom/](blitz-dom/README.md).
5. Use [leptos-native/](leptos-native/README.md), [ipc/](ipc/README.md), and the subsystem files under `leptos-native/` for subsystem details.
