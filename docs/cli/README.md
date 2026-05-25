# CLI

`crates/cli` provides the developer-facing command-line tool for `leptos-native`. It is a build-time tool, not a runtime dependency.

## Binary Name

The binary is `leptos-native` (same as the umbrella crate). The CLI is published with the rest of the workspace.

## Commands

### `leptos-native new <name>`

Scaffold a new application.

Flags:

- `--backend webview|blitz` — pick the rendering backend. Default: `webview`.
- `--template counter|empty|tray-only` — pick a starter template. Default: `counter`.

Output: a Cargo project with `Cargo.toml` configured for the chosen backend, a `src/main.rs` matching the template, and an empty `assets/` directory.

### `leptos-native run`

Run the current application or a workspace example.

Flags:

- `--example <name>` — run the example under `examples/<name>`.
- `--release`.
- `--backend webview|blitz` — override the default backend feature set.

Behavior: invokes `cargo run` with the appropriate features and the asset preparation pipeline.

### `leptos-native build`

Compile without running.

Flags:

- `--release`
- `--backend webview|blitz`
- `--target <triple>` — cross-compile.

### `leptos-native bundle`

Produce a distributable.

Flags:

- `--format msi|nsis|dmg|deb|appimage|...`
- `--target <triple>`

Delegates to a single bundling library (e.g., `cargo-packager`). The framework does not reimplement OS-specific packaging.

### `leptos-native check`

Runs `cargo check` plus the repository-required `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`.

### `leptos-native asset prepare`

Manually run the WebView asset pipeline. Mostly used by the framework itself; useful when debugging shell or interpreter bundling.

## Asset Pipeline (WebView Only)

`crates/webview-dom` ships the JS interpreter as a static `&'static str` baked into the binary. The CLI optionally extends that with user assets:

1. Reads the framework-supplied asset manifest (interpreter, default stylesheet, runtime helpers).
2. Reads user-supplied assets from `assets/` if present.
3. Builds a single in-binary asset bundle exposed through Wry's `with_asynchronous_custom_protocol` (`lnative://asset/<key>`).

Users do not write JavaScript — the interpreter is internal to `crates/webview-dom`. User code stays in Rust.

## Project Layout Conventions

`leptos-native new` scaffolds:

```
my-app/
  Cargo.toml
  src/
    main.rs
    components/      (optional)
  assets/
    icon.png
    styles.css       (optional, webview backend)
  README.md
```

The CLI does not require this layout. It is a convention that simplifies the asset pipeline and bundler defaults.

## Backend Feature Flags

The scaffolded `Cargo.toml` carries the chosen backend as a feature:

```toml
[dependencies.leptos-native]
version = "0.1"
features = ["webview"]
```

`leptos-native run --backend blitz` adds `--no-default-features --features blitz` to the underlying Cargo invocation. The CLI never tries to enable both backends in one binary.

## Diagnostic Output

The CLI uses `tracing` with a structured formatter. `-v`/`-vv` increases the level. Failures print actionable suggestions for known causes: missing system WebView, missing GPU adapter for Blitz, missing Linux tray dependency.

## Why a CLI

- Hide the asset pipeline behind a single command so user `Cargo.toml` files stay small.
- Provide a stable bundling story without forcing every user to learn a separate packaging tool.
- Give scaffolds for the common shapes so users start with a working program.

The CLI is optional. Plain `cargo build` still works.

## Out of Scope

- Hot reload. Future work; tracked separately.
- A live dev server with auto-rebuild. Plain `cargo watch` plus `leptos-native run` is the starting point.
- A plugin system. Commands stay fixed until a real extension need appears.
