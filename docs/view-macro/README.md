# `crates/view-macro`

The `view-macro` crate provides two procedural macros: `view!` for building view trees and `#[component]` for declaring components. Both emit calls against `crates/render` types and never name a specific backend.

## Strategy: Vendor `leptos_macro`, Modify Only Emission Paths

`view-macro` is a **vendored fork** of upstream `leptos_macro`, not a reimplementation. Pachys took the same approach (its `common/macro/Cargo.toml` self-describes as "Vendored from leptos_macro; renamed and rewritten to emit __pachys_view::* paths"). We do the same.

### Why vendor

The bulk of `leptos_macro` (~5800 lines) is backend-agnostic infrastructure:

- HTML-flavored Rust syntax parsing for `view!`.
- The view AST plus span-preserving error reporting.
- `#[component]` parsing, `FooProps` builder generation, `#[prop(...)]` attribute support.
- Auxiliary macros (`slot`, `memo`, `slice`, `params`, `lazy`).
- Inert-element optimization (collapse fully-static subtrees into a single HTML string).

Reimplementing all of this is months of work for zero behavioral gain over upstream. Vendoring lets us inherit upstream bug fixes by periodic re-sync.

### What we change

The narrow modification surface is the **emission paths** — the fully-qualified type paths that `quote!` blocks reference. Every `::leptos::tachys::*` becomes `::leptos_native_render::*`.

Concretely, the change set is concentrated in `view/mod.rs` (~1960 LOC) at roughly these emission points:

| Upstream `leptos_macro` emits | `view-macro` emits |
|---|---|
| `::leptos::tachys::html::element::#tag()` | `::leptos_native_render::Element::new(stringify!(#tag))` |
| `::leptos::tachys::svg::#tag()` | `::leptos_native_render::Element::new_svg(stringify!(#tag))` |
| `::leptos::tachys::html::InertElement::new(#html)` | `::leptos_native_render::InertElement::new(#html)` |
| `::leptos::tachys::reactive_graph::OwnedView::new(...)` | `::leptos_native_render::OwnedView::new(...)` |
| `::leptos::tachys::view::iterators::StaticVec::from(...)` | `::leptos_native_render::StaticVec::from(...)` |
| `::leptos::tachys::view::static_types::Static::<#text>` | `::leptos_native_render::Static::<#text>` |
| `::leptos::tachys::html::doctype(#value)` | (removed — desktop apps have no doctype) |

About 20 `quote!` macro call sites in `view/mod.rs`, plus the equivalents in `component.rs`. Everything else — the parsers, the AST, the diagnostic plumbing — is unchanged.

### What we delete

A few features are SSR/hydration/island specific and have no meaning in a native desktop app. We delete them rather than maintain dead code:

- `::leptos::tachys::html::islands::*` paths (islands architecture).
- `RenderHtml` plumbing for SSR string output.
- Hydration markers and the `data-hk` attribute pipeline.
- The `leptos::component_view::View` SSR adapters.

This trims `component.rs` by roughly 30%. The remaining surface is exactly the parts a native renderer needs.

### Sync cadence

`view-macro` pins to a specific upstream `leptos_macro` version (recorded in `crates/view-macro/UPSTREAM_VERSION`). Re-syncs are a manual operation: pull upstream into a branch, replay our emission-path change set, run the test suite. Cadence target: every minor `leptos` release.

We do **not** depend on `leptos_macro` as a Cargo dependency. The code is copied into `crates/view-macro/src/`. Cargo dependency would not work because the upstream crate's `lib.rs` exports the macros that hardcode the wrong emission paths; replacing them post-publish is not possible.

## Responsibility

- Parse JSX-flavored markup inside `view! { ... }` and lower it to `crates/render` constructor calls.
- Parse `#[component] fn Foo(...) -> impl IntoView` and rewrite it into a function plus a typed `FooProps` builder.
- Stay free of any backend dependency.

## Dependencies

- `proc-macro2`, `syn`, `quote`, `prettyplease` — same as upstream `leptos_macro`.
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
leptos_native_render::Element::new("button")
    .attr("class", "primary")
    .attr("disabled", move || disabled.get())
    .on("click", move |_| count.update(|n| *n += 1))
    .child(move || format!("count: {}", count.get()))
```

### Lowering rules (inherited from `leptos_macro`)

- A bare tag (`<div>`) becomes `Element::new("div")`.
- A static attribute value (`class="x"`) passes a `&'static str`.
- A reactive attribute value (`disabled=move || ...`) passes a closure; the reactive bridge in `crates/render` turns it into a per-attribute effect.
- An `on:event=handler` becomes `.on("event", handler)`.
- A child expression in `{ ... }` is wrapped as a child. Closures become reactive children.
- A literal string child wraps in `Text::new`.
- Sibling children compose into a tuple — heterogeneous and stored at the type level.
- Fully-static subtrees collapse into a single `InertElement::new("<html>...")` for compile-time and runtime efficiency. This is the single biggest win the upstream macro buys us; do not regress it.

### Why HTML-flavored

Both DOM crates accept HTML tags and CSS-style attribute names. The webview backend forwards them through IPC verbatim. The Blitz backend forwards them to `blitz_dom::Document` which is itself an HTML document. The macro stays HTML-flavored to keep one shared mental model.

## `#[component]` Macro

Inherited from `leptos_macro::component` with the same emission-path retargeting:

```rust
#[component]
fn Counter(initial: i32, #[prop(optional)] step: Option<i32>) -> impl IntoView {
    /* ... */
}
```

The macro emits:

- A `CounterProps` struct with typed fields plus a builder.
- A function `fn Counter(props: CounterProps) -> impl IntoView`.
- Optional `children: Children<R>` parameter passes through if declared (where `Children<R>` is from `crates/render`).

### Property attributes (inherited)

- `#[prop(optional)]` — generates `Option<T>` field.
- `#[prop(into)]` — generates an `impl Into<T>` builder method.
- `#[prop(default = ...)]` — supplies a default.

These follow upstream Leptos conventions verbatim; users coming from Leptos read them without surprise.

## Hot Reload

Out of scope for the initial implementation. Upstream `leptos_macro` already has hot-reload hooks; we keep the hook points intact during vendoring so a future `leptos_native_hot_reload` can plug in without re-forking.

## Errors

Inherited from upstream — the diagnostic infrastructure is one of the most polished parts of `leptos_macro` and we do not touch it.

- Unknown attributes pass through as-is — the renderer decides what to do with them.
- Reserved-keyword identifiers in tag names are rejected at parse time.
- Mismatched closing tags produce a clear span-pinned error.

## What This Crate Does Not Own

- The view tree types themselves — those are in `crates/render`.
- The `Renderer` trait or any of its implementations.
- Component runtime behavior — only the syntactic transform.
- Hydration, islands, or SSR — deleted during vendoring.
