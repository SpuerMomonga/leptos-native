# Repository Layout

A Rust monorepo for `leptos-native`, an experimental native desktop framework built with Wry, Blitz, Tao, and Leptos's reactive primitives.

## Scope

The framework targets:

- multi-window desktop applications
- async runtime-driven execution
- system tray integration
- native Rust code execution (no WebAssembly)
- two rendering backends: WebView (`webview-dom`) and Blitz (`blitz-dom`)
- shared component code across both backends
- no browser API surface for application code

## Top-Level Rules

- Keep each crate name short — at most two hyphenated segments.
- One umbrella entry crate (`leptos-native`) for application-facing APIs.
- Reuse upstream Leptos crates (`reactive_graph`, `any_spawner`, optional `reactive_stores`) directly. Do not wrap them in re-export shells.
- The `Renderer` trait boundary is stable enough to justify two real DOM crates (`webview-dom`, `blitz-dom`). Other subsystems (windowing, tray, runtime wiring) stay inside `leptos-native` until reuse outside the umbrella appears.
- Prefer typed APIs over stringly typed channels.

## `crates/` Structure

| Crate | Responsibility |
|---|---|
| `crates/render` | `Renderer` trait, `Render<R>`, `Mountable<R>`, view tree node types, `IntoView`, the reactive-bridge `Render` impl for `FnMut() -> V`. Depends on upstream `reactive_graph`. |
| `crates/view-macro` | `view!` macro and `#[component]` macro emitting `crates/render` types. |
| `crates/ipc` | Typed IPC messages (mutation, event, control) used by `webview-dom`. |
| `crates/webview-dom` | Wry-based `Renderer` implementation. Owns the embedded JS interpreter and the IPC transport. Depends on `render`, `ipc`, `wry`, `tao`. |
| `crates/blitz-dom` | Blitz-based `Renderer` implementation. Owns the Vello paint pipeline and the wgpu surface plumbing. Depends on `render`, `blitz-dom` upstream, `vello`, `wgpu`, `tao`. |
| `crates/leptos-native` | Umbrella entry crate. `Application`, `App`, `Window`, `WindowBuilder`, `Tray`. Owns the Tao event loop, the Tokio runtime, the Tao-aware `any_spawner` executor, and the tray integration. Selects one DOM crate via mutually exclusive features. |
| `crates/cli` | Developer tool: scaffolding, asset pipeline, run helpers, packaging. |

## Why No `core` / `reactive` / `bridge` / `store` / `runtime`

- `core` removed: had no concrete responsibility. Shared types live where they are used.
- `reactive` removed: it was a thin re-export of upstream `reactive_graph`. Consumers depend on the upstream crate directly through `leptos-native`'s prelude.
- `bridge` removed: the reactive-bridge `Render` impl lives inside `crates/render`, not in a separate crate.
- `store` removed: applications use `reactive_stores` directly when they need it.
- `runtime` removed: Tokio + Tao loop wiring lives inside `crates/leptos-native`. Splitting it out would just create a circular dependency.

These crates can come back if a real reuse boundary emerges.

## Why Two DOM Crates

`webview-dom` and `blitz-dom` are kept separate (rather than feature-gated modules inside `leptos-native`) because:

- The `Renderer` trait is a stable seam — both DOM crates have the same shape and zero overlapping internals.
- Each pulls a distinct heavy dependency tree (Wry vs. Vello/wgpu). Keeping them separate avoids dragging both into every build.
- It matches the pattern from sibling projects (dioxus's `desktop` / `native`, pachys's per-platform `dom` crates).

## Dependency Direction

```
reactive_graph (upstream)
   |
   v
render -> view-macro

ipc -> webview-dom -+
                    |
blitz-dom ----------+-> leptos-native
```

- `webview-dom` depends on `ipc`. `blitz-dom` does not (no IPC layer needed).
- Both DOM crates depend on `render` and on `tao` (for raw window handles and the event loop's user-event channel type).
- `leptos-native` selects one DOM crate via features; only the selected one is compiled into a given binary.
- `cli` depends on workspace metadata and build helpers; runtime crates do not depend on `cli`.

## Package Naming

- Directory names match crate names (`render`, `view-macro`, `ipc`, `webview-dom`, `blitz-dom`, `leptos-native`, `cli`).
- Package names in `Cargo.toml` use the `leptos-native-` prefix where it disambiguates publishing. Examples: `leptos-native-render`, `leptos-native-view-macro`, `leptos-native-ipc`, `leptos-native-webview-dom`, `leptos-native-blitz-dom`, `leptos-native` (umbrella), `leptos-native-cli`.
- The directory `view-macro` (not `macro`) avoids the Rust 2018+ `macro` keyword and makes the role obvious.

## Examples

Runnable examples live in `examples/`. Conventions are in [examples.md](examples.md).

## CLI Package

`crates/cli` provides developer commands beyond plain Cargo. See [cli/README.md](cli/README.md).

The CLI is a developer tool, not a runtime dependency of applications.
