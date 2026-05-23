#![forbid(unsafe_code)]

mod application;
mod tray;
mod window;

pub use application::{App, Application};
pub use tray::{TrayIcon, TrayIconBuilder};
pub use window::{Window, WindowBuilder};

pub mod prelude {
    pub use crate::{App, Application, TrayIcon, TrayIconBuilder, Window, WindowBuilder};
}
