# `crates/view-macro`

The `view-macro` crate provides two procedural macros: `view!` for building view trees and `#[component]` for declaring components. Both emit calls against `crates/render` types and never name a specific backend.

## Responsibility

- Parse JSX-flavored markup inside `view! { ... }` and lower it to `crates/render` constructor calls.
- Parse `#[component] fn Foo(...) -> impl IntoView` and rewrite it into a function plus a typed `FooProps` builder.
- Stay free of any backend dependency.

## Dependencies

- `proc-macro2`, `syn`, `quote`.
- Tested against `crates/render`'s public API.

## `view!` Macro

Accepts an HTML-like syntax:

```rust
view! {
    <button class="primary"
            disabled=move || disabled.get()
            on:click=move |_| { count.update(|n| *n += 1) }>
        {move || format!("count: {}", count.get())}
    </button>
}
```

Lowers to constructor calls on `crates/render`:

```rust
render::Element::new("button")
    .attr("class", "primary")
    .attr("disabled", move || disabled.get())
    .on("click", move |_| count.update(|n| *n += 1))
    .child(move || format!("count: {}", count.get()))
```

### Lowering rules

- A bare tag (`<div>`) becomes `Element::new("div")`.
- A static attribute value (`class="x"`) passes a `&'static str`.
- A reactive attribute value (`disabled=move || ...`) passes a closure; the reactive bridge in `crates/render` turns it into a per-attribute effect.
- An `on:event=handler` becomes `.on("event", handler)`.
- A child expression in `{ ... }` is wrapped as a child. Closures become reactive children.
- A literal string child wraps in `Text::new`.
- Sibling children compose into a tuple — heterogeneous and stored at the type level, like pachys.

### Why HTML-flavored

Both DOM crates accept HTML tags and CSS-style attribute names. The webview backend forwards them through IPC verbatim. The Blitz backend forwards them to `blitz_dom::Document` which is itself an HTML document. The macro stays HTML-flavored to keep one shared mental model.

## `#[component]` Macro

```rust
#[component]
fn Counter(initial: i32, #[prop(optional)] step: Option<i32>) -> impl IntoView {
    /* ... */
}
```

The macro emits:

- A `CounterProps` struct with typed fields plus a builder.
- A function `fn Counter(props: CounterProps) -> impl IntoView`.
- Optional `children: Children<R>` parameter passes through if declared.

Properties become a typed `FooProps` struct so call sites are checked at compile time.

### Property attributes

- `#[prop(optional)]` — generates `Option<T>` field; absent at the call site means `None`.
- `#[prop(into)]` — generates an `impl Into<T>` builder method.
- `#[prop(default = ...)]` — supplies a default when omitted.

These follow upstream Leptos conventions; users coming from Leptos read them without surprise.

## Hot Reload

Out of scope for the initial implementation. The macro is structured so a future hot-reload path (similar to `leptos_hot_reload`) can hook into the lowering, but no work is done up front.

## Errors

- Unknown attributes pass through as-is — the renderer decides what to do with them.
- Reserved-keyword identifiers in tag names are rejected at parse time.
- Mismatched closing tags produce a clear span-pinned error.

## What This Crate Does Not Own

- The view tree types themselves — those are in `crates/render`.
- The `Renderer` trait or any of its implementations.
- Component runtime behavior — only the syntactic transform.

## Why Not Reuse `leptos_macro`

`leptos_macro` emits calls against `tachys` types, which bind to `web_sys::Element`. Targeting a renderer-generic view layer requires a different lowering. Pachys took the same approach and ships `common/macro` with a forked `view!`.
