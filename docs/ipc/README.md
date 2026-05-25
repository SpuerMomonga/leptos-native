# IPC Protocol

The IPC protocol ships renderer mutations from Rust to the JS shell inside the Wry WebView, and ships DOM events back from the JS shell to Rust. Used only by `crates/webview-dom`. `crates/blitz-dom` does not use IPC — it mutates and observes the document in-process.

The protocol types live in `crates/ipc`.

## Transport

Two channels:

- **Mutation channel (Rust → WebView)** — `wry::WebView::evaluate_script` for small batches; `with_asynchronous_custom_protocol` (`lnative://batch/<seq>`) for bulk transfers.
- **Event channel (WebView → Rust)** — `wry::WebView::with_ipc_handler`. The JS shell calls `window.ipc.postMessage(json)` with a serialized `IpcEvent`.

The dioxus-desktop sibling project uses a localhost WebSocket and a binary sledgehammer protocol for higher throughput. `leptos-native` starts with the simpler script-eval + ipc-handler pair because reactive fine-grained updates produce fewer mutations per tick than VDom diffs. The protocol surface is designed so transport can swap to WebSocket + binary later without changing message shapes.

## Mutation Protocol (Rust → WebView)

```rust
#[derive(Serialize, Deserialize)]
pub struct MutationBatch {
    pub window_id: WindowId,
    pub seq: u64,
    pub ops: Vec<Mutation>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Mutation {
    CreateElement      { id: ElementId, tag: String },
    CreateText         { id: ElementId, value: String },
    CreatePlaceholder  { id: ElementId },

    SetText            { id: ElementId, value: String },
    SetAttribute       { id: ElementId, name: String, value: String },
    RemoveAttribute    { id: ElementId, name: String },

    InsertNode         { parent: ElementId, child: ElementId, before: Option<ElementId> },
    RemoveNode         { parent: ElementId, child: ElementId },
    ClearChildren      { parent: ElementId },

    AddListener        { id: ElementId, event: String, listener: ListenerId },
    RemoveListener     { id: ElementId, listener: ListenerId },

    Discard            { id: ElementId },
}
```

- `ElementId(u64)` is allocated by `webview-dom`. The JS interpreter holds a `Map<u64, Node>` mirror.
- `seq` is monotonic per window; the JS interpreter applies batches in order.
- `ListenerId(u64)` identifies a listener so events can name it.
- `Discard` releases an ID once Rust no longer needs it.

The op set mirrors the `Renderer` trait in `crates/render` one-to-one. Adding a method only adds a variant.

## Event Protocol (WebView → Rust)

```rust
#[derive(Serialize, Deserialize)]
pub struct IpcEvent {
    pub window_id: WindowId,
    pub listener: ListenerId,
    pub event: EventPayload,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventPayload {
    Click   { button: u8, x: f32, y: f32, modifiers: u8 },
    Input   { value: String },
    Change  { value: String },
    KeyDown { key: String, code: String, modifiers: u8 },
    KeyUp   { key: String, code: String, modifiers: u8 },
    Pointer { kind: String, x: f32, y: f32, modifiers: u8 },
    Submit,
    Focus,
    Blur,
    Custom  { name: String, data: serde_json::Value },
}
```

The JS shell builds an `EventPayload` from the native DOM event and posts it. `webview-dom` dispatches to the listener by `ListenerId` — the registered `Box<dyn FnMut(IpcEvent)>` is the user's `on:event` handler from `view!`.

## Control Protocol

Out-of-band messages used during startup, logging, and liveness:

```rust
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Control {
    Ready { window_id: WindowId, version: String },
    Log   { level: String, message: String },
    Error { window_id: WindowId, message: String, stack: Option<String> },
    Pong  { seq: u64 },
}
```

- `Ready` — sent by the JS shell once the interpreter is initialized. The framework holds back the first mutation batch until `Ready` arrives.
- `Log` and `Error` — route into the framework's `tracing` subscriber.
- `Pong` — answers a `Ping` mutation; flush barrier and liveness check.

## Serialization

JSON via `serde_json` is the default. The protocol is designed so a binary encoding (`bincode` or sledgehammer-style framing) can replace JSON without touching message shapes.

Numeric IDs are `u64`. Strings are owned at the boundary; `webview-dom` can pass `Cow<'static, str>` internally and convert at serialize time.

## Backpressure and Ordering

- Each window has its own sequence space (`seq`).
- `webview-dom` batches mutations per Tao loop tick: every reactive flush appends to a per-window batch; the batch ships when the tick completes.
- The interpreter applies a batch atomically: every op succeeds, or the batch fails (the framework treats the window as crashed and unmounts it).
- For very large batches, `webview-dom` switches transport from script-eval to the asynchronous custom protocol. The threshold is a tuning knob, not a protocol concern.

## Versioning

`Control::Ready { version }` lets `webview-dom` reject a JS shell that does not match. `version` is a SemVer string identifying the interpreter contract — bumped according to these rules:

- **Major** bump: a breaking protocol change (a `Mutation` or `EventPayload` variant changes shape, or the interpreter requires Rust-side cooperation that older versions do not provide). Mismatched major fails window startup.
- **Minor** bump: a new variant added at either end. Mismatched minor logs a warning and continues — the side that doesn't recognize the variant ignores it.
- **Patch** bump: bug fixes, performance work, no protocol shape change.

The CLI bundles the JS shell at build time and stamps the matching version into `webview-dom` as a `&'static str` constant, so the comparison is compile-time-known on the Rust side.

## What This Protocol Does Not Cover

- The Blitz backend. It does not use IPC.
- Cross-window IPC. Application-level messaging belongs in `crates/leptos-native` and uses Tao user events, not the WebView's IPC channel.
- Asset loading. The CLI prepares a manifest of in-binary assets that the `lnative://asset/...` custom protocol serves; the manifest is its own concern.
