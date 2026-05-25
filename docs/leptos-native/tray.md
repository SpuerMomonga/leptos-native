# System Tray

System tray support lives in `crates/leptos-native::tray`, built on the `tray-icon` crate. It works with either backend.

## Goals

- One typed API for adding a tray icon and menu, used identically by `webview-dom` and `blitz-dom` applications.
- Tray events flow through the same Tao event loop as window events; no separate channel.
- Menu items are typed Rust values, not stringly typed identifiers.

## Public API

```rust
pub struct TrayBuilder { /* ... */ }

impl TrayBuilder {
    pub fn new() -> Self;

    pub fn icon(self, icon: Icon) -> Self;
    pub fn tooltip(self, tooltip: impl Into<String>) -> Self;
    pub fn title(self, title: impl Into<String>) -> Self;

    pub fn menu(self, menu: TrayMenu) -> Self;
    pub fn on_click(self, handler: impl FnMut(&App, TrayClick) + 'static) -> Self;

    pub fn build(self) -> Tray;
}

pub struct TrayMenu { /* ... */ }

impl TrayMenu {
    pub fn new() -> Self;
    pub fn item<I: Into<MenuItem>>(self, item: I) -> Self;
    pub fn separator(self) -> Self;
    pub fn submenu(self, label: impl Into<String>, menu: TrayMenu) -> Self;
}

pub enum MenuItem {
    Action {
        id: MenuId,
        label: String,
        enabled: bool,
        accelerator: Option<Accelerator>,
        handler: Box<dyn FnMut(&App) + 'static>,
    },
    CheckBox {
        id: MenuId,
        label: String,
        checked: ReadSignal<bool>,
        handler: Box<dyn FnMut(&App, bool) + 'static>,
    },
    Predefined(PredefinedMenuItem),
}

pub enum PredefinedMenuItem {
    Quit,
    Hide,
    Show,
    About,
    Separator,
}
```

`MenuId(String)` is user-supplied and stable. `Accelerator` describes a keyboard shortcut.

### Supporting types

```rust
/// Stable identifier for a menu item, supplied by the application.
/// Used to refer to a specific item from `MenuEvent` and from reactive bindings.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MenuId(pub String);

/// Tray click event payload. Forwarded from `tray_icon::TrayIconEvent`.
pub enum TrayClick {
    /// Single primary-button click on the tray icon itself.
    Left,
    /// Single secondary-button click — typically opens the menu on Windows/Linux.
    Right,
    /// Double primary-button click.
    DoubleLeft,
}

/// Keyboard accelerator, modeled on `tray_icon::menu::Accelerator`.
///
/// Created via the `accelerator!` macro re-exported by `leptos-native`,
/// which accepts e.g. `accelerator!("CmdOrCtrl+Shift+P")`.
pub struct Accelerator { /* ... */ }

/// Icon source. Constructed from a path, raw RGBA bytes, or platform-native handle.
pub struct Icon { /* ... */ }

impl Icon {
    pub fn from_path(path: impl AsRef<Path>) -> leptos_native::Result<Self>;
    pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> leptos_native::Result<Self>;
}
```

`Accelerator` and `Icon` are thin wrappers over `tray_icon`'s equivalents — they exist on the framework side so that user code does not depend on `tray-icon`'s concrete types directly. Their internals are not stable; treat them as opaque.

A tray is registered with `App::set_tray(tray)` during setup. Most apps need at most one tray.

## Reactive Menu State

`MenuItem::CheckBox::checked` takes a `ReadSignal<bool>`, so a checkbox automatically reflects reactive state. `enabled` and `label` accept reactive values too:

```rust
TrayMenu::new()
    .item(MenuItem::action("toggle", "Show window")
        .enabled_signal(window_state.is_minimized)
        .handler(|app| if let Some(w) = app.get_window(&"main".into()) { w.show(); }))
```

`crates/leptos-native::tray` subscribes to these signals when the tray builds and updates the underlying `tray_icon::menu::MenuItem` when they change.

## Event Routing

`tray-icon` has its own static event channels. `crates/leptos-native::application` registers callbacks at `Application::run` time that forward to a Tao `EventLoopProxy`:

```rust
TrayIconEvent::set_event_handler(Some(move |event| {
    proxy.send_event(UserEvent::Tray(event)).ok();
}));
MenuEvent::set_event_handler(Some(move |event| {
    proxy.send_event(UserEvent::Menu(event)).ok();
}));
```

The Tao loop dispatches `UserEvent::Tray` and `UserEvent::Menu` to the registered tray's handlers. Tray and menu activity stay on the main thread alongside window events.

## Lifecycle

1. `Application::run` initializes the static `tray-icon` event handlers exactly once.
2. `App::set_tray(tray)` stores the tray builder.
3. After window setup, the framework calls `tray.build()`, which creates the underlying `tray_icon::TrayIcon` and registers menu items.
4. While running, click and menu events flow through user events.
5. On shutdown, the tray drops, removing the icon.

## Backend Independence

Tray code does not call into `webview-dom` or `blitz-dom`. It runs whether the app uses WebView or Blitz, and whether the app has zero, one, or many windows.

A tray-only app is supported: `App::set_tray` is called in `setup`; no windows are registered; the application exits only on `App::quit()` or `MenuItem::Predefined(Quit)`.

## Platform Notes

- Windows: tray icons appear in the notification area. Menus respond to right-click.
- macOS: `tray-icon` integrates with the menu bar. Hiding the dock icon is an `Info.plist` (`LSUIElement`) concern, set at bundle time by `crates/cli`, not a runtime API.
- Linux: `tray-icon` requires `libayatana-appindicator` or the legacy `libappindicator`. The CLI scaffold flags the runtime dependency.

## What This Layer Does Not Cover

- Native menu bars (top-level `File`/`Edit` menus attached to a window). That is a separate API on `WindowBuilder` and out of scope for this document.
- Notifications. Tray notifications use a different system API and would be added separately.
- Indicator badges with rich content. The current API supports text and an icon; richer indicators come if a real use case appears.
