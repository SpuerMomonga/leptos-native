# `crates/render`

The `render` crate defines the renderer-generic UI primitive: the `Renderer` trait, the view tree, the `Render<R>`/`Mountable<R>` traits, and the reactive bridge that ties closures-over-signals to renderer mutations.

This is the layer pachys calls `common/render`. The framework writes its own version because upstream `tachys` 0.7+ hardcoded `Rndr = Dom` and is not usable as a renderer-generic library.

## Responsibility

- Define one `Renderer` trait that both DOM crates (`webview-dom`, `blitz-dom`) implement.
- Provide the view tree shape (`Element<R, ...>`, `Text<R>`, `Fragment<R, ...>`, `Show`, `For`, `Suspense`) that `view!` emits.
- Provide the reactive bridge: `Render<R> for FnMut() -> V` where `V: Render<R>`.
- Define cross-cutting types every backend shares: `ListenerHandle`, `Children<R>`, `IntoView<R>`.
- Stay free of any backend dependency. Only upstream `reactive_graph`.

## Dependencies

- `reactive_graph` (upstream) — signals, memos, effects, owners.
- Nothing else. No `wgpu`, no `wry`, no `web-sys`, no `blitz-dom`.

## The `Renderer` Trait

```rust
pub trait Renderer: Sized + 'static {
    type Node: Clone + 'static;
    type Element: AsRef<Self::Node> + Clone + 'static;
    type Text: AsRef<Self::Node> + Clone + 'static;
    type Placeholder: AsRef<Self::Node> + Clone + 'static;
    type Event: 'static;

    fn create_element(tag: &str) -> Self::Element;
    fn create_text(text: &str) -> Self::Text;
    fn create_placeholder() -> Self::Placeholder;

    fn set_text(node: &Self::Text, text: &str);
    fn set_attribute(node: &Self::Element, name: &str, value: &str);
    fn remove_attribute(node: &Self::Element, name: &str);

    fn insert_node(parent: &Self::Element, child: &Self::Node, before: Option<&Self::Node>);
    fn remove_node(parent: &Self::Element, child: &Self::Node);
    fn clear_children(parent: &Self::Element);

    fn add_event_listener(
        node: &Self::Element,
        name: &str,
        handler: Box<dyn FnMut(Self::Event) + 'static>,
    ) -> ListenerHandle;
}
```

The trait is HTML-flavored. Both DOM crates accept HTML element tag names (`div`, `button`, `span`) and CSS-style attribute names. `webview-dom` forwards them through IPC; `blitz-dom` interprets them through `blitz_dom::Document`. `view!` syntax is identical across backends.

`Renderer` is **not** `Send`. The trait carries no instance state — methods are associated functions — so `Send` would be decoration, not a real bound. Per-backend per-window state lives in backend-owned structs (`WebViewBackend`, `BlitzBackend`) and stays on the main thread.

### Multi-Window Backend Routing

The `Renderer` methods are static; they need to know *which window's backend* to mutate. The framework solves this with a per-tick scoped thread-local:

```rust
// In each DOM crate.
thread_local! {
    static BACKEND: RefCell<Option<BackendRef>> = RefCell::new(None);
}

pub fn with_backend<R>(backend: &BackendRef, f: impl FnOnce() -> R) -> R {
    BACKEND.with(|slot| {
        let prev = slot.borrow_mut().replace(backend.clone());
        let result = f();
        *slot.borrow_mut() = prev;   // restore, never just clear
        result
    })
}
```

**Routing rule:** every entry point that calls `Renderer` methods sets `BACKEND` to the right window first, runs the work, then restores the previous value. The entry points are:

1. **Initial mount** — `Window::open` flow calls `with_backend(&window.backend, || mount_to(root, view))`.
2. **Reactive re-runs** — the reactive bridge captures the active backend at `build()` time and re-installs it on every `rebuild()`. See "The Reactive Bridge" below.
3. **Event dispatch** — when an event arrives for a window, `dispatch_event` runs handlers under `with_backend(&that_window.backend, ...)`. User handlers usually update signals; if those signal updates trigger effects whose subscribers belong to a *different* window, point 2 takes over.

The save+restore (rather than save+clear) matters: an effect for Window A may, while running, set a signal that triggers an effect for Window B; B's effect re-installs B's backend; on its return, we want A back, not nothing.

This is the same shape pachys uses, just made explicit. The previous draft of this doc described the thread-local but did not specify cross-window correctness — that was a bug.

`ListenerHandle` is a small RAII struct returned by `add_event_listener`; dropping it removes the listener. The view layer holds the handle inside the mountable state.

## Cross-Cutting Types

Defined once in `crates/render`, reused by every backend:

```rust
/// RAII handle returned by `Renderer::add_event_listener`. Dropping it
/// removes the underlying listener via the renderer.
pub struct ListenerHandle {
    inner: Box<dyn FnOnce() + 'static>,
}

impl ListenerHandle {
    pub fn new(remove: impl FnOnce() + 'static) -> Self;
    pub fn forget(self);                     // Leak the listener for the program's lifetime.
}

impl Drop for ListenerHandle {
    fn drop(&mut self) { /* call self.inner */ }
}

/// Component children. Wraps a thunk that produces a renderer-generic view.
pub struct Children<R: Renderer> {
    inner: Box<dyn FnOnce() -> AnyView<R> + 'static>,
}

impl<R: Renderer> Children<R> {
    pub fn new<V: IntoView<R> + 'static>(view: impl FnOnce() -> V + 'static) -> Self;
    pub fn into_view(self) -> AnyView<R>;
}

/// Type-erased view, used by `Children<R>` and any place that needs to store
/// a heterogeneous tree behind a single type.
pub struct AnyView<R: Renderer> { /* trait-object backed */ }

impl<R: Renderer> Render<R> for AnyView<R> { /* ... */ }

pub trait IntoView<R: Renderer> {
    type View: Render<R>;
    fn into_view(self) -> Self::View;
}
```

`Children<R>` is a thunk, not a `Vec` — children are evaluated inside the parent component's `Owner` so reactive scope is correct. `AnyView<R>` is the type-erasure escape hatch; tuple-typed children are still preferred when the structure is statically known.

## `Render<R>` and `Mountable<R>`

```rust
pub trait Render<R: Renderer>: Sized {
    type State: Mountable<R>;

    fn build(self) -> Self::State;
    fn rebuild(self, state: &mut Self::State);
}

pub trait Mountable<R: Renderer> {
    fn unmount(&mut self);
    fn mount(&mut self, parent: &R::Element, marker: Option<&R::Node>);
    fn insert_before_this(&self, child: &mut dyn Mountable<R>) -> bool;
}
```

- `Render<R>` describes how a value becomes mounted state for renderer `R`.
- `Mountable<R>` is the state side: it knows how to insert itself, remove itself, and report its boundaries for sibling diffing.

`Render<R>::State == Mountable<R>` is what makes incremental updates possible. Same shape as pachys `common/render/src/render.rs`.

## The Reactive Bridge

Any `FnMut() -> V` where `V: Render<R>` is itself `Render<R>`:

```rust
impl<R, F, V> Render<R> for F
where
    R: Renderer,
    F: FnMut() -> V + 'static,
    V: Render<R>,
{
    type State = RenderEffectState<V::State, R>;

    fn build(mut self) -> Self::State {
        let backend = current_backend::<R>();          // captured at build time
        RenderEffect::new(move |prev| {
            with_backend(&backend, || {
                let value = self();
                match prev {
                    Some(mut state) => { value.rebuild(&mut state); state }
                    None => value.build(),
                }
            })
        }).into()
    }
    // rebuild similar
}
```

This is the load-bearing piece. When the closure reads a signal, the effect subscribes; on signal change, the effect re-runs and `rebuild` updates only the renderer state that changed. The captured `backend` makes sure the right window's renderer state is mutated, even if the effect fires from a signal owned elsewhere.

`RenderEffect` from `reactive_graph::effect` runs synchronously on its first call, so a freshly mounted view renders immediately without waiting for the executor.

## View Tree Types

Exposed node kinds:

- `Element<R, Children, Attrs, Listeners>` — typed element with HTML tag, attribute tuple, listener tuple, and child tuple.
- `Text<R>` — wraps a string-yielding value.
- `Fragment<R, Children>` — a sequence with no element wrapper.
- `Placeholder<R>` — empty marker used by conditionals.
- `Show<R, Cond, Then, Else>` — `if`-style conditional.
- `For<R, Items, Key, Render>` — keyed list rendering.
- `Suspense<R, Fallback, Body>` — async boundary backed by a `Resource`.

Each implements `Render<R>` for any `R: Renderer`.

## Mounting

```rust
pub fn mount_to<R, V>(root: R::Element, view: V) -> MountedView<R, V::State>
where
    R: Renderer,
    V: Render<R>;
```

`MountedView` owns the root state and the `Owner` for the subtree. Dropping it unmounts.

User code does not call `mount_to` directly; `crates/leptos-native::WindowBuilder` calls it from inside its window setup, wrapped in `with_backend`.

## Compile-Time Budget

Renderer-generic view trees with heterogeneous tuple children are exactly what made upstream `tachys` move to `Rndr = Dom`: the type machinery exploded compile times. We accept the same risk in exchange for two real backends. Mitigations:

- Cap tuple children arity at 16 (same limit pachys uses). Beyond that, users wrap in a `Fragment`.
- Provide `AnyView<R>` as an opt-in escape hatch — large lists, conditionally-typed sections, and component children erase to `AnyView<R>` rather than producing deeply nested generics.
- The `view!` macro uses inert HTML strings (`InertElement`) for fully-static subtrees, side-stepping the typed-tuple machinery for the common-case "static markup with a few reactive holes" pattern. This is the single biggest win in upstream `leptos_macro`.
- We track `cargo build --timings` on the `counter` example as a regression gate: any commit that pushes it above a budget (TBD when we have a baseline) is rejected.

If the type-system cost still becomes prohibitive, the fallback is to specialize the view tree on a single concrete `R` per binary using a type alias defined by `crates/leptos-native`, treating the `Renderer` trait as a plug-in surface rather than a generic parameter on every type. We hold this in reserve.

## Testing: `MockRenderer`

A `MockRenderer` lives in `crates/render` under `#[cfg(any(test, feature = "test-util"))]`. It implements `Renderer` against an in-process tree of `Rc<RefCell<MockNode>>` and exposes assertions:

```rust
#[cfg(feature = "test-util")]
pub mod test {
    pub struct MockRenderer;
    impl Renderer for MockRenderer { /* ... */ }

    pub fn assert_html(root: &MockElement, expected: &str);
    pub fn dispatch(node: &MockElement, event: MockEvent);
}
```

This lets framework crates and downstream applications unit-test components without booting Tao or a GPU. Pachys does not ship one and pays for it; we do, from day one.

## What This Crate Does Not Own

- The choice of renderer — `crates/leptos-native` business.
- Event semantics for a specific backend — defined by `R::Event`.
- The `view!` macro and `#[component]` macro — those live in `crates/view-macro`.
- Async coordination and the Tao-aware executor — those live in `crates/leptos-native`.

## Why Not Reuse `tachys`

Upstream `tachys` 0.7+ defines `pub type Rndr = dom::Dom;` unconditionally and threads `web_sys::Element` through every type. The renderer-generic version was removed for compile-time reasons. Reintroducing the generic upstream is out of scope; pachys took the same view and forked. We write a narrower replacement (~1.5k LOC target) that only carries what the `view!` macro needs.
