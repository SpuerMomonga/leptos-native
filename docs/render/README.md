# `crates/render`

The `render` crate defines the renderer-generic UI primitive: the `Renderer` trait, the view tree, the `Render<R>`/`Mountable<R>` traits, and the reactive bridge that ties closures-over-signals to renderer mutations.

This is the layer pachys calls `common/render`. The framework writes its own version because upstream `tachys` 0.7+ hardcoded `Rndr = Dom` and is not usable as a renderer-generic library.

## Responsibility

- Define one `Renderer` trait that both DOM crates (`webview-dom`, `blitz-dom`) implement.
- Provide the view tree shape (`Element<R, ...>`, `Text<R>`, `Fragment<R, ...>`, `Show`, `For`, `Suspense`) that `view!` emits.
- Provide the reactive bridge: `Render<R> for FnMut() -> V` where `V: Render<R>`.
- Stay free of any backend dependency. Only upstream `reactive_graph` and the macro crate.

## Dependencies

- `reactive_graph` (upstream) — signals, memos, effects, owners.
- Nothing else. No `wgpu`, no `wry`, no `web-sys`, no `blitz-dom`.

## The `Renderer` Trait

```rust
pub trait Renderer: Sized + Send + 'static {
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

`ListenerHandle` is a small RAII struct returned by `add_event_listener`; dropping it removes the listener. The view layer holds the handle inside the mountable state.

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
        RenderEffect::new(move |prev| {
            let value = self();
            match prev {
                Some(mut state) => { value.rebuild(&mut state); state }
                None => value.build(),
            }
        }).into()
    }
    // rebuild similar
}
```

This is the load-bearing piece. When the closure reads a signal, the effect subscribes; on signal change, the effect re-runs and `rebuild` updates only the renderer state that changed.

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

`IntoView<R>` is a convenience trait so any value that can become a view tree can be returned from a component.

## Mounting

```rust
pub fn mount_to<R, V>(root: R::Element, view: V) -> MountedView<R, V::State>
where
    R: Renderer,
    V: Render<R>;
```

`MountedView` owns the root state and the `Owner` for the subtree. Dropping it unmounts.

User code does not call `mount_to` directly; `crates/leptos-native::WindowBuilder` calls it from inside its window setup.

## What This Crate Does Not Own

- The choice of renderer — `crates/leptos-native` business.
- Event semantics for a specific backend — defined by `R::Event`.
- The `view!` macro and `#[component]` macro — those live in `crates/view-macro`.
- Async coordination and the Tao-aware executor — those live in `crates/leptos-native`.

## Why Not Reuse `tachys`

Upstream `tachys` 0.7+ defines `pub type Rndr = dom::Dom;` unconditionally and threads `web_sys::Element` through every type. The renderer-generic version was removed for compile-time reasons. Reintroducing the generic upstream is out of scope for this project; pachys took the same view and forked. We write a narrower replacement (~1.5k LOC target) that only carries what the `view!` macro needs.
