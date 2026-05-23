# Application

`Application` is the public entry point for the `native` subproject.

## Responsibility

`Application` owns the app-level runtime surface for application-facing APIs:

- builder setup
- application state and handles
- window registration and lookup
- event loop startup
- integration points for tray, menu, and backend selection

It should stay the primary host abstraction in `crates/native`.

## API Design

The public API should stay small and centered on `Application`, `App`, and window construction.

### Core Types

- `Application`: the top-level runtime object.
- `App`: the runtime-facing handle used during setup and event handling.
- `WindowBuilder`: creates windows and attaches mounted views.
`Application` should provide its own default startup behavior, so callers can start with `Application::default()` or `Application::new()` and then customize only the parts they need.

### Expected Shape

```rust
Application::default()
    .setup(|app| {
        let window = WindowBuilder::new()
            .id("main")
            .title("leptos-native")
            .mount(Counter)
            .build();

        app.set_window(window);
    })
    .run();
```

### API Rules

- Prefer `Application::default()` or `Application::new()` as the entry point.
- Keep setup callbacks focused on registration and initialization.
- Expose explicit methods for window registration, lookup, and lifecycle control.
- Keep backend-specific switches visible in the API name or config.
- Prefer structured commands over stringly typed control messages.

## Boundary

`Application` coordinates subsystems, but it should not absorb their internal logic.

- `runtime` owns async runtime integration and event-loop orchestration when that remains separate.
- `window` logic stays focused on window creation and window lifecycle.
- `tray` logic stays focused on tray/menu wiring.
- `ipc` and `bridge` define structured communication, not application policy.
- `core` should remain backend-agnostic and hold shared contracts only.

## Lifecycle

1. Build an `Application` with typed configuration.
2. Register setup callbacks or initial services.
3. Create windows through the application-facing builder API.
4. Start the event loop.
5. Dispatch application events through explicit commands or reactive state.

## Design Notes

- Prefer small, typed builders over loose argument lists.
- Keep backend-specific behavior explicit in names and module boundaries.
- Preserve reactive state flow for Leptos integration.
- Keep `Application` as the stable public surface while backend internals evolve.
