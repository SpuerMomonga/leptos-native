# Examples

Examples live under `examples/`. Each example is a runnable Cargo binary in its own crate.

## Example Conventions

- One example per directory. The directory name matches the crate name.
- Each example has its own `Cargo.toml`, registered as a workspace member in the root `Cargo.toml`.
- Examples depend on `leptos-native` with explicit feature flags (`webview` or `blitz`). One example demonstrates one backend; if both are interesting, ship two examples.
- Examples must build with `cargo clippy --workspace --all-targets -- -D warnings`.
- Each example has a short `README.md` explaining what it shows and the run command.

## Recommended Examples

| Example | Backend | Demonstrates |
|---|---|---|
| `counter` | webview | minimal app, signal, button click, derived value |
| `counter-blitz` | blitz | the same counter component on Blitz |
| `multi-window` | webview | opening, focusing, closing additional windows |
| `tray` | webview | tray icon, menu, action handler, hide-to-tray |
| `tokio-task` | webview | spawning a Tokio task that updates a signal |
| `form-input` | webview | text input, submit, two-way binding patterns |
| `routing` | webview | client-side routing inside a single window |
| `theming` | both | switching backends with the same component code |

The set grows as the framework matures. Each example stays small (one to three screens of code).

## Anatomy of an Example

```rust
use leptos_native::prelude::*;

#[component]
fn App() -> impl IntoView {
    let (count, set_count) = signal(0);
    view! {
        <main class="container">
            <h1>"counter"</h1>
            <button on:click=move |_| set_count.update(|n| *n += 1)>
                {move || format!("clicked {} times", count.get())}
            </button>
        </main>
    }
}

fn main() {
    Application::default()
        .setup(|app| {
            let main_window = WindowBuilder::new("main", App)
                .title("counter")
                .inner_size(480, 320)
                .resizable(true)
                .build();
            app.open_window(main_window);
        })
        .run();
}
```

The pattern:

- One `#[component] fn App()` returning `impl IntoView`.
- One `main` that builds an `Application`, registers windows in `setup`, and calls `run`.
- The example's `Cargo.toml` selects exactly one backend feature.

## Cross-Backend Examples

To demonstrate that the same code runs on both backends:

- Put shared components in a small library crate (`examples/<name>/lib.rs` or a dedicated workspace member).
- Provide two binary entry points (`bin/webview.rs`, `bin/blitz.rs`) that pick a backend feature.
- Run with `cargo run -p <name> --bin webview` or `--bin blitz`.

This keeps each binary feature-clean and avoids feature-gated `main` functions in a single binary.

## What Examples Should Not Do

- Do not depend on the CLI's asset pipeline unless the CLI is the subject of the example. Use plain Cargo.
- Do not introduce backend-specific code in shared component bodies. Isolate per-backend features behind feature gates.
- Do not import from `crates/webview-dom` or `crates/blitz-dom` directly. Examples use the public `leptos_native::prelude` only.

## Example as Specification

When the public API changes, the `counter` example is the canonical reference. If the example breaks, the change is incomplete; if the example becomes verbose, the API regressed. The example doubles as a spec.
