# AGENTS.md

## Project Notes

- `leptos-native` is an experimental native framework built with Wry and Leptos.
- Main capabilities: multi-window support, async runtime integration, system tray support, native Rust execution, no browser APIs, and WebView rendering.
- Project overview and public examples live in [README.md](README.md).
- Repository structure details live in [docs/repository.md](docs/repository.md).

## Subproject Rules

- Keep crate directory names to at most two words.
- Put reusable framework code under `crates/`.
- Put runnable demos under `examples/`.
- Use `crates/native` as the public entry crate for application-facing APIs.
- Keep WebView and future Skia backend code inside `native` internals until a real reuse boundary appears.
- Add a new crate only when the responsibility is stable and cannot stay as a module inside an existing crate.

## Rust Style

- Forbid unsafe code in new crates unless the need is explicit and documented.
- Prefer small modules with clear ownership over broad utility modules.
- Prefer typed configuration structs and builders over loosely typed argument lists.
- Prefer structured IPC messages over string-based ad hoc protocols.
- Keep Leptos state flow reactive, using signals, memos, and effects for their intended roles.
- Keep event handlers focused on updating signals or dispatching explicit commands.
- Do not hide backend-specific behavior behind vague names; name WebView, Skia, tray, and windowing boundaries directly.

## Working Rules

- Read the relevant workspace, crate, and module files before making changes.
- Keep changes scoped to the smallest useful boundary.
- Preserve the separation between runtime, windowing, tray, IPC, state, and backend code.
- Do not couple business logic directly to a specific rendering backend when a shared abstraction will do.

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
