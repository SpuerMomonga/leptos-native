# CLI

`crates/cli` provides the developer-facing command-line tool for `leptos-native`. It is a build-time tool, not a runtime dependency.

## Binary Name

The binary is **`cargo-leptos-native`**, installed alongside Cargo and invoked as a Cargo subcommand:

```bash
cargo install leptos-native-cli
cargo leptos-native new my-app
cargo leptos-native run
cargo leptos-native bundle --format msi
```

The Cargo subcommand convention (matching `cargo-leptos`, `cargo-tauri`, `cargo-binstall`) avoids naming collisions with the `leptos-native` umbrella crate that applications import. A user's `Cargo.toml` says `leptos-native = "0.1"`; their command line says `cargo leptos-native run`. The two never refer to the same thing.

## Commands

### `cargo leptos-native new <name>`

Scaffold a new application.

Flags:

- `--backend webview|blitz` — pick the rendering backend. Default: `webview`.
- `--template counter|empty|tray-only` — pick a starter template. Default: `counter`.

Output: a Cargo project with `Cargo.toml` configured for the chosen backend, a `src/main.rs` matching the template, and an empty `assets/` directory.

### `cargo leptos-native run`

Run the current application or a workspace example.

Flags:

- `--example <name>` — run the example under `examples/<name>`.
- `--release`.
- `--backend webview|blitz` — override the default backend feature set.

Behavior: invokes `cargo run` with the appropriate features and the asset preparation pipeline.

### `cargo leptos-native build`

Compile without running.

Flags:

- `--release`
- `--backend webview|blitz`
- `--target <triple>` — cross-compile.

### `cargo leptos-native bundle`

Produce a distributable.

Flags:

- `--format msi|nsis|dmg|deb|appimage|...`
- `--target <triple>`

Delegates to a single bundling library (e.g., `cargo-packager`). The framework does not reimplement OS-specific packaging.

### `cargo leptos-native check`

Runs `cargo check` plus the repository-required `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`.

### `cargo leptos-native asset prepare`

Manually run the asset pipeline. Mostly used by the framework itself; useful when debugging shell, interpreter, or asset bundling.

## Project Configuration: `[package.metadata.leptos-native]`

The CLI reads project configuration from `[package.metadata.leptos-native]` in the application's `Cargo.toml`. This piggy-backs on Cargo's existing config file rather than introducing a separate `leptos-native.toml` — the schema is small and adding a second file would not pay for itself. If/when configuration grows past ~10 fields, a dedicated file becomes worthwhile.

```toml
[package.metadata.leptos-native]
id = "com.example.counter"            # required: reverse-DNS application identifier
display-name = "Counter"              # optional: overrides Cargo's `package.name` for UI/bundle display
icon = "assets/icon.png"              # optional: window/dock/taskbar icon
single-instance = false               # optional: future single-instance lock support

[package.metadata.leptos-native.assets]
# Asset manifest — see "Asset Manifest" below.
include = ["assets/**"]
exclude = ["assets/raw/**"]

[package.metadata.leptos-native.bundle]
# Per-format bundle config; all keys optional, all delegated to the underlying bundler.
identifier = "com.example.counter"    # defaults to `id` above
publisher = "Example, Inc."
copyright = "© 2026 Example, Inc."
category = "Utility"
```

A `build.rs` shipped by `crates/leptos-native` reads this metadata at build time (via `cargo metadata`) and bakes the relevant pieces in as `&'static str` constants. `Application::id` consults this constant when the application code does not call `Application::id(...)` explicitly.

## Asset Pipeline

`crates/webview-dom` ships the JS interpreter as a static `&'static str` baked into the binary. The CLI's asset pipeline extends that with user assets, shipped to both backends through a shared in-binary store:

1. Reads the framework-supplied asset manifest (interpreter for `webview-dom`, default stylesheet, runtime helpers).
2. Reads user-supplied assets per `[package.metadata.leptos-native.assets]` from `Cargo.toml`.
3. Builds a single in-binary asset bundle exposed:
   - To `webview-dom` through Wry's `with_asynchronous_custom_protocol` (`lnative://asset/<key>`).
   - To `blitz-dom` through a synchronous in-memory loader registered on `blitz_dom::Document`.

Users do not write JavaScript — the interpreter is internal to `crates/webview-dom`. User code stays in Rust.

### Asset Manifest

`[package.metadata.leptos-native.assets]` selects which files become part of the bundle:

```toml
[package.metadata.leptos-native.assets]
# Glob patterns evaluated relative to the crate root.
include = ["assets/**", "fonts/*.woff2"]
exclude = ["assets/raw/**", "**/*.psd"]

# Optional: rewrite paths in the bundle. Default: keep the source path
# rooted at `assets/` (e.g. `assets/icon.png` becomes `lnative://asset/icon.png`).
[[package.metadata.leptos-native.assets.alias]]
from = "fonts/Inter-Regular.woff2"
to   = "fonts/inter.woff2"
```

The manifest's job is to declare *what* is bundled, not *how*. Compression, hashing, and content-type detection are CLI internals.

The CLI emits a generated `assets.manifest.bin` next to the build artifacts; `crates/leptos-native`'s `build.rs` reads it via `OUT_DIR` and `include_bytes!`s it in. Users do not interact with this file.

### Inline `include_bytes!` is also fine

For tiny applications, the manifest is overkill. `include_bytes!` and `include_str!` against files in the crate continue to work — the asset pipeline is opt-in. The manifest pays for itself once an app has more than a handful of assets or wants the same path to work across both backends.

## Backend Feature Flags

The scaffolded `Cargo.toml` carries the chosen backend as a feature:

```toml
[dependencies.leptos-native]
version = "0.1"
features = ["webview"]
```

`cargo leptos-native run --backend blitz` adds `--no-default-features --features blitz` to the underlying Cargo invocation. The CLI never tries to enable both backends in one binary.

## Diagnostic Output

The CLI uses `tracing` with a structured formatter. `-v`/`-vv` increases the level. Failures print actionable suggestions for known causes: missing system WebView, missing GPU adapter for Blitz, missing Linux tray dependency.

## Why a CLI

- Hide the asset pipeline behind a single command so user `Cargo.toml` files stay small.
- Provide a stable bundling story without forcing every user to learn a separate packaging tool.
- Give scaffolds for the common shapes so users start with a working program.

The CLI is optional. Plain `cargo build` still works — assets fall back to inline `include_bytes!` and bundling becomes the user's problem.

## Out of Scope

- Hot reload. Future work; tracked separately.
- A live dev server with auto-rebuild. Plain `cargo watch` plus `cargo leptos-native run` is the starting point.
- A plugin system. Commands stay fixed until a real extension need appears.
