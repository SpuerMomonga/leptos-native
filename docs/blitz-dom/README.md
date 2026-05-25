# `crates/blitz-dom`

The `blitz-dom` crate is the Blitz backend: a `Renderer` implementation that mutates an in-process `blitz_dom::Document` directly and paints it via Vello onto a wgpu surface attached to a Tao window.

This is one of the two real backend crates. It is mutually exclusive with `crates/webview-dom` — exactly one is selected per `leptos-native` build via the umbrella's features.

The crate name `blitz-dom` deliberately mirrors the upstream `blitz-dom` crate it depends on, since it is a thin renderer adapter on top of that engine.

## Responsibility

- Implement `render::Renderer` against `blitz_dom::NodeId`.
- Bootstrap the per-window `blitz_dom::Document` from an HTML shell.
- Own the wgpu surface and the Vello paint pipeline for the window.
- Translate Tao input events into Blitz events and dispatch hits to listeners.
- Expose a per-window `BlitzBackend` that `crates/leptos-native` attaches to a Tao window.

## Dependencies

- `crates/render` — for the `Renderer` trait.
- `blitz-dom` (upstream) — the in-process DOM engine.
- `blitz-html` (upstream) — to bootstrap a document from HTML.
- A Blitz renderer crate (e.g., `blitz-renderer-vello`) — for paint.
- `vello`, `wgpu` — GPU paint and surface management.
- `tao` — for the window handle and event-loop proxy.
- `raw-window-handle` — for surface creation.

No `crates/ipc` dependency. The Blitz backend has no IPC layer.

## Architecture

For each window:

1. `crates/leptos-native` creates the Tao `Window`.
2. `BlitzBackend::attach(window, config, proxy)`:
   - Derives `RawWindowHandle` and `RawDisplayHandle` from the Tao window.
   - Builds a `wgpu::Surface` against a shared `wgpu::Instance`/`Adapter`/`Device`.
   - Bootstraps a `blitz_dom::Document` from `HtmlShell`.
   - Creates a `BlitzRenderer` (Vello-based) targeted at the surface.
3. The renderer (`BlitzDomRenderer`) maps `R::Element` to `blitz_dom::NodeId` and applies tree mutations directly on the document.
4. Reactive effects mutate the document and mark it dirty.
5. After the Tao tick, dirty windows call `tao::window::Window::request_redraw`.
6. On `WindowEvent::RedrawRequested`, the framework runs Blitz layout if needed and paints a frame.
7. Tao input events translate to Blitz events and dispatch into the document, which bubbles them to the framework's listeners.

There is no IPC, no JavaScript, no separate process.

## Layout

```
crates/blitz-dom/
  src/
    lib.rs              -- public entry: BlitzBackend, BlitzDomRenderer
    renderer.rs         -- impl Renderer for BlitzDomRenderer
    backend.rs          -- per-window state
    document.rs         -- HTML shell -> Document bootstrap
    paint.rs            -- per-frame paint pipeline (Vello + wgpu)
    surface.rs          -- shared wgpu Instance/Adapter/Device + per-window Surface
    input.rs            -- Tao WindowEvent -> Blitz event translation
    listener.rs         -- listener registration and dispatch
```

## Renderer Implementation

```rust
pub struct BlitzDomRenderer;

impl render::Renderer for BlitzDomRenderer {
    type Node = BlitzNode;
    type Element = BlitzElement;
    type Text = BlitzText;
    type Placeholder = BlitzPlaceholder;
    type Event = BlitzEvent;

    fn create_element(tag: &str) -> Self::Element {
        BACKEND.with(|b| {
            let id = b.document_mut().create_element(tag);
            BlitzElement { id }
        })
    }

    // ...similar for create_text, create_placeholder, set_text, set_attribute,
    //    remove_attribute, insert_node, remove_node, clear_children, add_event_listener
}
```

`BACKEND` is a thread-local pointing at the active window's `BlitzBackend`, set when a paint or mutation cycle begins. Same pattern as `webview-dom` and as pachys.

`BlitzNode`, `BlitzElement`, `BlitzText`, `BlitzPlaceholder` are thin newtypes around `blitz_dom::NodeId`.

## Per-Window Backend State

```rust
pub struct BlitzBackend {
    pub window_id: WindowId,
    pub tao_window: Arc<tao::window::Window>,
    pub document: RefCell<blitz_dom::Document>,
    pub viewport: RefCell<blitz_traits::Viewport>,
    pub renderer: RefCell<BlitzPaintRenderer>,
    pub surface: wgpu::Surface,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub listeners: RefCell<HashMap<ListenerId, Box<dyn FnMut(BlitzEvent) + 'static>>>,
    pub dirty: Cell<bool>,
}
```

`BlitzPaintRenderer` wraps the upstream Blitz renderer (e.g., `blitz_renderer_vello::VelloRenderer`).

The `wgpu::Instance`/`Adapter` are shared across windows; each window has its own `Surface`, `Device`, and `Queue`.

## Surface Bootstrap

```rust
fn create_surface(
    window: &tao::window::Window,
    instance: &wgpu::Instance,
) -> Result<wgpu::Surface, BackendError> {
    let target = wgpu::SurfaceTarget::Window(Box::new(window.clone()));
    instance.create_surface(target)
}
```

Tao windows expose `RawWindowHandle` and `RawDisplayHandle` which is what wgpu needs.

## Mutation Flow

When a reactive effect re-runs and calls `BlitzDomRenderer::set_attribute`:

1. Look up the active window's document via `BACKEND`.
2. Call `document.set_attribute(node_id, name, value)`.
3. Mark `dirty = true` on the window.

After the Tao tick, batch redraw requests:

```rust
pub fn after_tick(&self) {
    for window in &self.windows {
        if window.dirty.replace(false) {
            window.tao_window.request_redraw();
        }
    }
}
```

A tick with 50 mutations produces at most one redraw per window.

## Paint Pipeline

On `WindowEvent::RedrawRequested`:

1. Resolve layout (Taffy via `blitz_dom`) if needed.
2. Build a Vello `Scene` from the document tree using the upstream Blitz renderer.
3. Acquire a surface texture, render the scene, present.

Reusing the upstream Blitz renderer crate means the framework doesn't reimplement painting.

## Input Translation

`tao::event::WindowEvent` converts to `BlitzEvent`:

- `CursorMoved` → `PointerMove { x, y }`.
- `MouseInput` → `PointerButton { kind, button }`.
- `KeyboardInput` → `Key { key, code, modifiers }`.
- `Resized(size)` → updates the viewport, reconfigures the surface, marks dirty, requests redraw.

Once Blitz resolves hit-testing and bubbling, the framework gets a per-listener notification and calls the user-supplied handler. The user handler typically updates a signal, which schedules the next reactive cycle.

## Public API

```rust
pub struct BlitzBackend { /* ... */ }

impl BlitzBackend {
    pub fn attach(
        window: Arc<tao::window::Window>,
        config: BlitzConfig,
        proxy: tao::event_loop::EventLoopProxy<UserEvent>,
    ) -> Result<Self, BackendError>;

    pub fn renderer_root(&self) -> BlitzElement;          // The document's <body> or root mount.
    pub fn handle_window_event(&self, event: &WindowEvent);
    pub fn paint(&self);                                  // Called on RedrawRequested.
    pub fn detach(self);
}

pub struct BlitzConfig {
    pub shell: HtmlShell,
    pub stylesheet: Option<String>,
    pub font_ctx: Option<Arc<blitz_traits::FontContext>>,
}
```

## Limitations

- Blitz's CSS support is a tracked subset of the spec. Documents that depend on full browser CSS may not render identically. The supported subset is documented alongside Blitz's own docs.
- Blitz is in active development; pinning a Blitz version per `leptos-native` release matters.
- A compatible GPU adapter must be available. The framework surfaces a clear error if not.

## What This Crate Does Not Own

- Application-level concerns (windows, tray, runtime). Those are in `crates/leptos-native`.
- The `Renderer` trait. Defined in `crates/render`.
- IPC. There is none for Blitz.
- Event loop policy. Owned by `crates/leptos-native`; this crate just reports events through the user-event channel.
