# Tech Stack

This document is organized by technology first, then by the APIs each technology contributes.

## Wry

Wry is the WebView layer.

Use these parts:

- `WebViewBuilder` to create and configure a WebView.
- `WebView` as the runtime object for the embedded web content.
- `with_url` or `with_html` to choose the initial renderer source.
- `with_ipc_handler` to receive structured messages from the renderer.
- `with_initialization_script` to inject startup scripts.
- `with_new_window_req_handler` to control renderer-initiated window requests.
- `build(&window)` to attach the WebView to a native window.

Used for:

- application UI rendering
- renderer-to-Rust messaging
- window-scoped web content startup

## Tao

Tao is the native windowing and event-loop layer.

Use these parts:

- `EventLoop` and `EventLoopBuilder` to own the desktop event loop.
- `EventLoopProxy` to dispatch custom events into the loop.
- `ControlFlow` to manage loop behavior.
- `WindowBuilder` to create windows.
- `Window` to represent the native host window.
- `WindowEvent` to react to window lifecycle and input events.

Used for:

- creating application windows
- managing window lifecycle
- routing native events
- integrating app-level commands with the event loop

## tray-icon

`tray-icon` is the system tray and tray menu layer.

Use these parts:

- `TrayIconBuilder` to create a tray icon.
- `TrayIcon` to own the tray instance.
- `TrayIconEvent` to observe tray interactions.
- `menu::Menu` to define tray menus.
- `menu::MenuItem` and `menu::IconMenuItem` for actionable menu entries.
- `menu::PredefinedMenuItem` for standard commands.
- `menu::MenuEvent` for menu selection handling.

Used for:

- system tray integration
- tray menus and menu actions
- app commands surfaced outside the window

## Leptos

Leptos is the reactive UI layer.

Use these parts:

- `component` for component definitions.
- `view` for JSX-like view construction.
- `signal` and `RwSignal` for reactive state.
- `Memo` for derived values.
- `Effect` for side effects.
- `batch` for grouped reactive updates.
- `IntoView` for component return types.

Used for:

- mounted UI composition
- local and shared reactive state
- derived UI state
- effect-driven integration with application state

## Tokio

Tokio is the async runtime layer.

Use these parts:

- `runtime::Builder` to configure the runtime.
- `Runtime` to own runtime execution.
- `tokio::spawn` for async tasks.
- `spawn_blocking` for blocking work.
- `LocalSet` for non-`Send` task coordination when needed.

Used for:

- background work
- async services
- task orchestration around the application loop

## How The Stack Maps To The Framework

- `crates/native` should wrap Tao, Wry, and tray-icon behind `Application`, `App`, and `WindowBuilder`.
- `crates/signals` should wrap Leptos reactive primitives.
- `crates/ipc` should define structured renderer communication.
- `crates/bridge` should connect Rust state to renderer state.
- `crates/runtime` should own Tokio integration if it stays separate from `native`.
- `crates/core` should stay backend-agnostic and hold shared contracts only.

## Design Constraints

- Keep windowing and tray APIs explicit instead of hiding them behind vague host names.
- Keep backend-specific details in `native` internals until reuse justifies extraction.
- Prefer typed Rust APIs over stringly typed control channels.
- Keep the stack narrow until a real boundary appears.
