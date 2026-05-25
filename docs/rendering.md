# Rendering Backends

`leptos-native` ships two rendering backend crates. Both implement the [`Renderer` trait](render/README.md) defined in `crates/render`.

| Backend crate | Feature flag (on `leptos-native`) | Renders via | Hosts |
|---|---|---|---|
| `webview-dom` | `webview` | system WebView (WebView2 / WKWebView / WebKitGTK) | Wry |
| `blitz-dom` | `blitz` | native paint (Vello over wgpu, Parley + Taffy via Blitz) | Blitz |

Exactly one backend is selected per binary. The features are mutually exclusive at compile time.

## When To Pick Which

Pick **`webview-dom`** when:

- The app benefits from CSS-rich styling, animations, and DOM ecosystem (chart libraries, code editors, rich text).
- Distribution size matters more than runtime independence.
- Browser DevTools attached to your UI is valuable.
- You can rely on a system WebView being installed.

Pick **`blitz-dom`** when:

- The app must avoid a system WebView dependency (consistent rendering across machines, no WebView2 install on Windows).
- A fully self-contained binary with no out-of-process renderer is required.
- The UI is a clean HTML/CSS subset that does not need DOM-only features.
- A tighter Rust-only profile suits the deployment (kiosk, single-purpose tools).

Both backends accept the same `view!` and `#[component]` code. Switching is a feature flag change, not a rewrite.

## Common Behavior

Across both backends:

- `R::Element` corresponds to an HTML-style element (`div`, `button`, `span`, etc.).
- Attributes are CSS-style strings. Inline styles use `style`. Classes use `class`.
- Events follow DOM conventions: `click`, `input`, `pointerdown`, etc.
- Mounting attaches a view subtree to the window's root element (`<div id="root">`).
- Reactive effects schedule on the Tao main thread.
- Each window has exactly one root element and one mounted view subtree.

## `webview-dom` Backend

See [webview-dom/README.md](webview-dom/README.md) for full internals.

In one paragraph: Wry hosts a WebView attached to a Tao window. At startup, the framework loads a small HTML/JS shell containing an interpreter for a typed mutation protocol. Reactive effects produce typed `Mutation` values; `WebViewBackend` queues them per tick and ships each batch to the interpreter, which applies it to the live DOM. DOM events flow back through Wry's IPC handler as `IpcEvent` payloads, dispatched to the right listener.

Critical pieces: typed `Mutation` enum (see [ipc/README.md](ipc/README.md)), JS interpreter shipped as a `&'static str`, `WebViewElementId(u64)` mirrored on the JS side.

## `blitz-dom` Backend

See [blitz-dom/README.md](blitz-dom/README.md) for full internals.

In one paragraph: a `blitz_dom::Document` lives in the Rust process. The renderer maps `R::Element` to `blitz_dom::NodeId` and applies mutations directly. Each Tao window has an associated wgpu surface; on each redraw, Blitz paints the document via Vello onto that surface. Tao input events translate into Blitz's input model and dispatch into the document, where they bubble back up to listeners and end as signal updates.

Critical pieces: `blitz_dom::Document` per window, Taffy for layout, Parley for text, Vello + wgpu for paint.

## Switching Backends

```toml
[dependencies]
leptos-native = { version = "0.1", features = ["webview"] }
# or
leptos-native = { version = "0.1", default-features = false, features = ["blitz"] }
```

Application code is identical:

```rust
use leptos_native::prelude::*;

#[component]
fn App() -> impl IntoView {
    let (count, set_count) = signal(0);
    view! {
        <button on:click=move |_| set_count.update(|n| *n += 1)>
            {move || format!("count: {}", count.get())}
        </button>
    }
}

fn main() {
    Application::default()
        .setup(|app| {
            let main_window = WindowBuilder::new("main", App)
                .title("counter")
                .inner_size(640, 480)
                .build();
            app.open_window(main_window);
        })
        .run();
}
```

## Compatibility Surface

Backends agree on:

- HTML tag names for elements.
- The CSS subset accepted by Blitz (the WebView side always supports more).
- Standard DOM event names.
- Attribute conventions (`class`, `style`, `id`, `data-*`).

Backends differ on:

- CSS support: WebView is a full browser engine; Blitz is a tracked subset.
- Web platform APIs: only the WebView backend exposes them, and `leptos-native` does not promote their use.
- DevTools: WebView only.
- Binary size and startup: Blitz is heavier in binary size; WebView is heavier in startup latency on cold-start systems.

The framework's portable code path is the intersection. Stick to it for cross-backend components; reach for backend-specific features only inside backend-specific modules.

## Future Backends

A third backend implements `crates/render::Renderer` and slots in next to the existing two as a new crate (`skia-dom`, `tui-dom`, etc.) plus a feature on `leptos-native`. The `Renderer` trait is the only contract.
