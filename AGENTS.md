# AGENTS.md

## Project Notes

- `leptos-native` is an experimental native framework built with Wry, Blitz, Tao, and Leptos's reactive primitives.
- Main capabilities: multi-window support, async runtime integration, system tray support, native Rust execution, no browser APIs, and two interchangeable rendering backends (WebView and Blitz).
- Project overview and public examples live in [README.md](README.md).
- Repository structure details live in [docs/repository.md](docs/repository.md).
- Architecture overview lives in [docs/architecture.md](docs/architecture.md).

## Subproject Rules

- Keep crate directory names short (at most two hyphenated segments).
- The umbrella entry crate is `crates/leptos-native`. Application-facing APIs (`Application`, `App`, `Window`, `WindowBuilder`, `Tray`) live there.
- The `Renderer` trait in `crates/render` is the only seam between view code and backends. Anything that varies per backend lives below it.
- Two real backend crates: `crates/webview-dom` (Wry) and `crates/blitz-dom` (Blitz). They are alternatives, not coexistents.
- Reuse upstream Leptos crates (`reactive_graph`, `any_spawner`, optional `reactive_stores`) directly. Do not wrap them in re-export shells.
- Do not depend on upstream `tachys`, `leptos_dom`, or `leptos_macro` — they hardcode the DOM renderer. Use `crates/render` and `crates/view-macro` instead.
- Add a new crate only when the responsibility is stable and cannot stay as a module inside an existing crate.

## Rust Style

- Forbid unsafe code in new crates unless the need is explicit and documented.
- Prefer small modules with clear ownership over broad utility modules.
- Prefer typed configuration structs and builders over loosely typed argument lists.
- Prefer structured IPC messages over string-based ad hoc protocols.
- Keep Leptos state flow reactive, using signals, memos, and effects for their intended roles.
- Keep event handlers focused on updating signals or dispatching explicit commands.
- Do not hide backend-specific behavior behind vague names; name WebView, Blitz, tray, and windowing boundaries directly.

## Working Rules

- Read the relevant workspace, crate, and module files before making changes.
- Keep changes scoped to the smallest useful boundary.
- Preserve the separation between event loop, windowing, tray, IPC, render, and backend code.
- Do not couple business logic directly to a specific rendering backend when the `Renderer` trait already abstracts it.

## Required Checks

After writing Rust code, run these commands from the repository root:

```shell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

For documentation-only changes, run at least:

```shell
cargo metadata --no-deps --format-version 1
```
