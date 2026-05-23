# Repository Layout

This repository is a Rust monorepo for an experimental native framework built with Wry and Leptos.

## Scope

The framework targets:

- multi-window applications
- async runtime-driven execution
- system tray integration
- native Rust code execution
- no browser API surface
- WebView rendering

## Top-Level Rules

- Keep each crate name to at most two words.
- Prefer one host-style entry package for application-facing APIs.
- Keep reusable primitives in small shared crates instead of spreading them across the host crate.
- Treat WebView, tray, windowing, menu, IPC, and future Skia rendering as internal concerns of the native host unless they must be reused elsewhere.

## `crates/` Structure

Use this layout as the default workspace shape:

- `crates/native`: the public entry crate; owns the builder API, application handle, window creation, event loop startup, WebView host integration, tray/menu support, and future Skia backend selection.
- `crates/core`: shared framework contracts, app metadata, and common types that should stay independent from platform backends.
- `crates/signals`: Leptos-backed reactive state helpers, signals, memos, and effects.
- `crates/ipc`: IPC message types, serialization, and protocol helpers.
- `crates/bridge`: fine-grained sync between Rust-side state and the renderer.
- `crates/store`: domain stores and shared data access helpers.
- `crates/runtime`: async runtime integration and event-loop orchestration, only if it stays distinct from `native`.
- `crates/cli`: project build tool for scaffolding, running examples, bundling assets, packaging apps, and future developer commands.

## Dependency Direction

Preferred dependency flow:

`core` -> `signals`

`core` -> `ipc` -> `bridge` -> `native`

`native` -> `runtime`

`native` -> `store`

`cli` may depend on workspace metadata and build-time helpers, but runtime crates should not depend on `cli`.

WebView and Skia implementations should sit behind `native` internals or feature-gated modules until a real reuse boundary appears.

## Package Naming

- Use short, concrete names.
- Keep names stable and descriptive of responsibility.
- Example package names: `native`, `core`, `signals`, `ipc`, `bridge`, `store`, `runtime`, `cli`.

## CLI Package

The repository should include `crates/cli` once build workflows need more than plain Cargo commands.

Initial CLI responsibilities:

- create new app or example scaffolds
- run examples with the right feature set
- prepare WebView assets
- package native applications
- provide future project diagnostics

The CLI is a developer tool, not a framework runtime dependency.
