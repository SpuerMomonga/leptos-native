# leptos-native

`leptos-native` is an experimental native framework built with Wry and Leptos.

It focuses on:

- multi-window support
- async runtime integration
- system tray support
- native Rust code execution
- no browser APIs
- WebView-based rendering

## Getting Started

The startup shape in `leptos-native` is: create the application, create windows from a window builder, mount components through `App`, then run the event loop.

```rust
use leptos_native::prelude::*;

#[component]
fn Counter() -> impl IntoView {
    let (count, set_count) = signal(0);
    let doubled = memo(move |_| count.get() * 2);

    effect(move |_| {
        tracing::info!("main window count = {}", count.get());
    });

    view! {
        <section class="counter">
            <button on:click=move |_| set_count.update(|value| *value += 1)>
                "Increment"
            </button>
            <button on:click=move |_| set_count.update(|value| *value -= 1)>
                "Decrement"
            </button>
            <p>"Count: " {move || count.get()}</p>
            <p>"Doubled: " {move || doubled.get()}</p>
        </section>
    }
}

#[component]
fn Tools() -> impl IntoView {
    view! {
        <section class="tools">
            <p>"Tools window"</p>
        </section>
    }
}

fn main() {
    Application::default()
        .setup(|app| {
            let main_window = WindowBuilder::new("main", Counter)
                .title("leptos-native")
                .inner_size(960, 640)
                .resizable(true)
                .build();
            app.open_window(main_window);

            let tools_window = Window::builder("tools", Tools)
                .title("Tools")
                .inner_size(360, 520)
                .resizable(false)
                .build();
            app.open_window(tools_window);
        })
        .run();
}
```

## Layout

- `docs/`: design notes, architecture, and specifications.
- `crates/`: reusable Rust crates for runtime, IPC, UI primitives, and backend adapters.
- `examples/`: runnable example applications.
- Repository structure details: [docs/repository.md](docs/repository.md)

## Workspace

Cargo workspace packages are listed explicitly in the root `Cargo.toml` workspace members.
