# `crates/webview-dom`

The `webview-dom` crate is the WebView backend: a `Renderer` implementation that ships mutations to a Wry-hosted WebView through IPC and receives DOM events back.

This is one of the two real backend crates. It is mutually exclusive with `crates/blitz-dom` — exactly one is selected per `leptos-native` build via the umbrella's features.

## Responsibility

- Implement `render::Renderer` against an in-memory shadow tree of `WebViewElementId(u64)`.
- Build per-window IPC transport on top of Wry.
- Embed the JS interpreter that applies typed mutations to the live DOM.
- Translate inbound `IpcEvent` messages into listener invocations.
- Expose a per-window `WebViewBackend` that `crates/leptos-native` attaches to a Tao window.

## Dependencies

- `crates/render` — for the `Renderer` trait.
- `crates/ipc` — for `Mutation`, `IpcEvent`, `Control` message types.
- `wry` — the WebView host.
- `tao` — for the window handle and event-loop proxy.
- `serde`, `serde_json` — IPC serialization.

## Architecture

For each window:

1. `crates/leptos-native` creates the Tao `Window`.
2. `WebViewBackend::attach(window, config, proxy)` builds a `wry::WebView` with:
   - `with_html(shell.into_html())` for the HTML shell.
   - `with_initialization_script(INTERPRETER_JS)` for the embedded interpreter.
   - `with_ipc_handler(...)` to receive `IpcEvent` and `Control` messages.
   - `with_asynchronous_custom_protocol("lnative", ...)` for bulk transfers and bundled assets.
3. The renderer (`WebViewRenderer`) keeps a counter that allocates `WebViewElementId`s and queues typed `Mutation` values per tick.
4. After each Tao tick, the queue flushes as a `MutationBatch`; the JS interpreter applies it.
5. Inbound events arrive through Wry's IPC handler; they are forwarded to the Tao loop as a custom user event and dispatched to the right listener.

## Layout

```
crates/webview-dom/
  src/
    lib.rs              -- public entry: WebViewBackend, WebViewRenderer
    renderer.rs         -- impl Renderer for WebViewRenderer
    backend.rs          -- per-window state (queue, listeners, ready flag)
    shell.rs            -- HtmlShell builder
    interpreter.js      -- the embedded JS interpreter (compile-time str)
    transport.rs        -- mutation flush, event ingest
    protocol.rs         -- the lnative:// asynchronous custom protocol
```

## Renderer Implementation

```rust
pub struct WebViewRenderer;

impl render::Renderer for WebViewRenderer {
    type Node = WebViewNode;
    type Element = WebViewElement;
    type Text = WebViewText;
    type Placeholder = WebViewPlaceholder;
    type Event = ipc::IpcEvent;

    fn create_element(tag: &str) -> Self::Element {
        let id = WebViewElementId::next();
        BACKEND.with(|b| b.queue_mut().push(ipc::Mutation::CreateElement {
            id, tag: tag.into()
        }));
        WebViewElement { id }
    }

    // ...similar for create_text, create_placeholder, set_text, set_attribute,
    //    remove_attribute, insert_node, remove_node, clear_children, add_event_listener
}
```

`BACKEND` is a thread-local pointing at the active window's `WebViewBackend`. It is set when a paint or mutation cycle begins for that window. This pattern matches pachys per-platform `Dom` thread-locals and keeps every renderer call argument-light.

`WebViewNode`, `WebViewElement`, `WebViewText`, `WebViewPlaceholder` are thin newtypes around `WebViewElementId`. Zero runtime cost.

## Per-Window Backend State

```rust
pub struct WebViewBackend {
    pub window_id: WindowId,
    pub tao_window: Arc<tao::window::Window>,
    pub webview: wry::WebView,
    pub queue: RefCell<Vec<ipc::Mutation>>,
    pub seq: Cell<u64>,
    pub listeners: RefCell<HashMap<ipc::ListenerId, Box<dyn FnMut(ipc::IpcEvent) + 'static>>>,
    pub ready: Cell<bool>,
    pub pending: RefCell<Vec<ipc::MutationBatch>>,
}
```

`queue` is the in-progress batch. `pending` holds batches that arrive before the JS interpreter signals `Ready`. `seq` is the per-window monotonic batch sequence.

## Mutation Flush

Hooked into Tao's per-tick boundary (after `MainEventsCleared`):

```rust
fn flush(&self) {
    if self.queue.borrow().is_empty() { return; }
    let ops = std::mem::take(&mut *self.queue.borrow_mut());
    let seq = self.seq.update(|s| { *s += 1; *s });
    let batch = ipc::MutationBatch { window_id: self.window_id.clone(), seq, ops };
    if !self.ready.get() {
        self.pending.borrow_mut().push(batch);
        return;
    }
    let json = serde_json::to_string(&batch).unwrap();
    if json.len() < INLINE_THRESHOLD {
        let _ = self.webview.evaluate_script(&format!("window.__lnInterpreter.apply({})", json));
    } else {
        self.serve_batch(batch);
    }
}
```

`INLINE_THRESHOLD` defaults to ~64 KB. Larger batches go through the asynchronous custom protocol (`lnative://batch/<seq>`) so the JS engine doesn't parse a megabyte of inline script.

## Event Reception

```rust
let webview = wry::WebViewBuilder::new_as_child(&tao_window)
    .with_html(shell.into_html())
    .with_initialization_script(INTERPRETER_JS)
    .with_ipc_handler(move |request| {
        match serde_json::from_str::<IncomingMessage>(request.body()) {
            Ok(IncomingMessage::Event(e)) => proxy.send_event(UserEvent::WebViewEvent(window_id.clone(), e)).ok(),
            Ok(IncomingMessage::Control(c)) => proxy.send_event(UserEvent::WebViewControl(window_id.clone(), c)).ok(),
            Err(err) => tracing::warn!(?err, "bad IPC message"),
        };
    })
    .with_asynchronous_custom_protocol("lnative".into(), {
        let proxy = proxy.clone();
        move |request, responder| { /* dispatch to protocol.rs */ }
    })
    .build()?;
```

The `proxy` is the Tao `EventLoopProxy<UserEvent>` from `crates/leptos-native`. Custom user events deliver IPC messages back to the main thread, where `WebViewBackend::dispatch_event` looks up the listener by `ipc::ListenerId` and runs the handler.

## Embedded Interpreter (`interpreter.js`)

A small JS module baked into the binary as a `&'static str`. It contains:

- A `Map<u64, Node>` for `WebViewElementId → live DOM Node`.
- An `apply(batch)` function that interprets each `Mutation` variant.
- Event registration that wraps each DOM listener with `window.ipc.postMessage(JSON.stringify(...))`.
- A `Ready` ping sent on `DOMContentLoaded`.

The interpreter is intentionally featureless — a translator, not a framework. See [../ipc/README.md](../ipc/README.md) for the wire protocol.

## Public API

```rust
pub struct WebViewBackend { /* ... */ }

impl WebViewBackend {
    pub fn attach(
        window: Arc<tao::window::Window>,
        config: WebViewConfig,
        proxy: tao::event_loop::EventLoopProxy<UserEvent>,
    ) -> Result<Self, BackendError>;

    pub fn renderer_root(&self) -> WebViewElement;     // The window's <div id="root">
    pub fn flush_mutations(&self);                     // Called after each tick.
    pub fn dispatch_event(&self, event: ipc::IpcEvent);// Called from the event loop.
    pub fn handle_control(&self, msg: ipc::Control);   // Ready, Log, Error, Pong.
    pub fn detach(self);                               // Drops the WebView.
}

pub struct WebViewConfig {
    pub shell: HtmlShell,
    pub stylesheet: Option<String>,
    pub initial_url: Option<String>,
}
```

`UserEvent` is re-exported from `crates/leptos-native` (a single user-event type spans the whole loop; backend variants are part of it).

## Limitations

- One `WebView` per window. Not multiplexed.
- The system WebView must be available (WebView2 on Windows, WKWebView on macOS, WebKitGTK on Linux). The CLI scaffold flags this dependency.
- DOM-only APIs (canvas, WebGL inside the page, browser-specific APIs) are reachable from JS but the framework does not promote them — they break Blitz portability.

## What This Crate Does Not Own

- Rendering. The system WebView paints itself.
- Layout. The browser engine handles it.
- Application-level concerns (windows, tray, runtime). Those are in `crates/leptos-native`.
- The `Renderer` trait. Defined in `crates/render`.
- Event loop policy. Owned by `crates/leptos-native`; this crate just reports events through the user-event channel.
