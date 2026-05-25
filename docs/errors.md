# Errors

`leptos-native` uses one error type across every fallible boundary in the framework: `leptos_native::Error`, an alias for `anyhow::Error`. Every public `Result` in the public API uses this type. There is no `WindowError`, no `BackendError`, no `IconError` — those have been folded into the single error.

## Why anyhow

The framework crosses many error sources: Tao OS errors, wgpu adapter failures, Wry IPC errors, `tray-icon` registration errors, font loading, asset bundle decode, IO. Defining a typed enum that unions all of these means a constantly growing `From` impl set and an outer enum users still cannot exhaustively match on (the inner variants stay open). `anyhow::Error` collapses the union, preserves the cause chain, and lets us add context as we propagate. For the small number of failures users actually want to *match* on (not just log), we define typed sentinels that callers can downcast. This is the same shape `tauri::Error` is moving toward and the standard pattern in modern Rust application frameworks.

`thiserror` stays available for sentinel types — see "Downcasting to known kinds" below — but it is not the public surface.

## Public types

```rust
// In `leptos-native::error`, re-exported through the prelude.

pub use anyhow::{Error, Context};

pub type Result<T, E = Error> = std::result::Result<T, E>;
```

The prelude exports `Error`, `Result`, and `Context`. `Context` is the trait that gives `Result<T, E>` and `Option<T>` the `.context(...)` and `.with_context(...)` methods. `bail!` and `ensure!` are also re-exported.

```rust
use leptos_native::prelude::*;

fn load_icon(path: &Path) -> Result<Icon> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("read icon {}", path.display()))?;
    Icon::from_bytes(&bytes).context("decode icon")
}
```

## Where errors surface

Every public fallible call returns `leptos_native::Result<T>`. The set is small:

- `WindowBuilder::build() -> Result<Window>` — Tao window creation, backend attach, mount.
- `Icon::from_path(...) -> Result<Icon>`, `Icon::from_rgba(...) -> Result<Icon>`.
- `TrayBuilder::build() -> Result<Tray>` — only when called outside `Application::run`'s setup or if the OS rejects the tray registration; building during setup does not return a `Result` because the framework defers the system call to the loop and surfaces failures the same way.
- `App::spawn` / `App::spawn_local` return `tokio::task::JoinHandle<F::Output>`; the `JoinHandle` itself is fallible at await time. The framework does not wrap Tokio's `JoinError`.
- Asset and bundle helpers in `crates/cli` return `Result<...>` where applicable.

Internal fallible calls (Tao, wgpu, Wry, tray-icon) attach context with `.with_context(|| ...)` as they propagate so the final error chain reads top-down: `window 'main' failed to open` → `backend attach failed` → `wgpu: no compatible adapter`.

## Downcasting to known kinds

For the small number of failures users want to *handle* (not just log), the framework defines typed sentinels in `leptos_native::error::kind`:

```rust
pub mod kind {
    use thiserror::Error;
    use crate::WindowId;

    #[derive(Debug, Error)]
    #[error("window with id {0:?} is already registered")]
    pub struct DuplicateWindow(pub WindowId);

    #[derive(Debug, Error)]
    #[error("system WebView is not installed (Windows: WebView2; Linux: WebKitGTK)")]
    pub struct WebViewMissing;

    #[derive(Debug, Error)]
    #[error("no compatible GPU adapter found")]
    pub struct GpuAdapterMissing;

    #[derive(Debug, Error)]
    #[error("Linux tray dependency missing (libayatana-appindicator)")]
    pub struct TrayDependencyMissing;

    #[derive(Debug, Error)]
    #[error("invalid window geometry: {0}")]
    pub struct InvalidGeometry(pub &'static str);
}
```

Match by downcast:

```rust
use leptos_native::error::kind;

match WindowBuilder::new(app, "main", App).build() {
    Ok(window) => { /* ... */ }
    Err(err) if err.is::<kind::WebViewMissing>() => {
        show_install_webview_dialog();
    }
    Err(err) if err.is::<kind::GpuAdapterMissing>() => {
        show_gpu_required_dialog();
    }
    Err(err) => {
        tracing::error!(?err, "window failed to open");
    }
}
```

The `kind` set stays small. A new sentinel is added only when users have a concrete reason to branch on it — typically a recoverable startup failure that surfaces in UI. Generic IO and serialization errors stay anonymous inside the `anyhow::Error`.

## Error chain conventions

When propagating, attach context at every layer that adds meaning:

```rust
// backend layer
let surface = instance
    .create_surface(target)
    .context("create wgpu surface for window")?;

// window layer
let backend = BlitzBackend::attach(&tao_window, config, proxy)
    .with_context(|| format!("attach Blitz backend for window {:?}", id))?;

// builder layer
let window = WindowBuilder::new(app, id, component)
    .build()
    .with_context(|| format!("open window {:?}", id))?;
```

The resulting `Display` output reads like a stack of intentions, not a stack of frames. `Debug` output preserves the underlying source chain (`anyhow::Error::chain`) for `tracing::error!` use.

## Panic vs Err policy

The framework distinguishes between programmer errors and runtime failures:

**Panic** is reserved for programmer errors — invariants the framework cannot recover from and the user can prevent by reading the docs:

- `Renderer` methods called outside `enter_scope`.
- `use_window()` called outside a window's component tree.
- `provide_context::<T>` followed by `use_context::<T>` of a different type.
- A component panicking inside `view!`. The panic propagates to the window's mount and aborts that window's bring-up; other windows stay alive.

**`Result`** is used for runtime conditions the user can plausibly handle:

- OS resource failures (window creation, GPU adapter, WebView availability).
- IO failures (icon load, asset load).
- Configuration mismatches caught at build time (e.g. `min > max` inner size returns `kind::InvalidGeometry`).

The framework never silently swallows errors. Async tasks that error without a result destination log at `error` level on the `leptos_native` target.

## Logging integration

Errors that the framework cannot return to a user call site (because they happen on background work the user did not await — tray dispatch, IPC parse, late wakeup failures) are emitted through `tracing::error!` with structured fields:

```rust
tracing::error!(
    target = "leptos_native::tray",
    error = ?err,
    "tray menu event dispatch failed"
);
```

`?err` formats the entire `anyhow::Error` chain. See [logging.md](logging.md) for the target naming convention and recommended subscriber setup.

## What this design is not

- It is not a typed error enum. Users that prefer typed errors at module boundaries can build their own on top with `thiserror` — the framework will not stand in their way.
- It is not a re-export of `eyre` or `color-eyre`. The framework standardizes on `anyhow` to match the rest of the Rust async ecosystem (tokio, axum, reqwest) without adding a second error library to the dependency tree.
- It does not panic on failed window bring-up. A failed `WindowBuilder::build` returns `Err` and leaves nothing registered. The application keeps running and can retry, fall back, or exit on its own terms.
