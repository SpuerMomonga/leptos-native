#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex};

use runtime::Runtime;

use crate::window::Window;

#[derive(Debug, Clone)]
pub struct Application {
    app: App,
    runtime: Runtime,
}

#[derive(Debug, Clone, Default)]
pub struct App {
    windows: Arc<Mutex<Vec<Window>>>,
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}

impl Application {
    pub fn new() -> Self {
        Self {
            app: App::default(),
            runtime: Runtime::new(),
        }
    }

    pub fn setup<F>(self, setup: F) -> Self
    where
        F: FnOnce(&App),
    {
        setup(&self.app);
        self
    }

    pub fn run(mut self) {
        self.runtime.start();
    }
}

impl App {
    pub fn set_window(&self, window: Window) {
        self.windows
            .lock()
            .expect("app state poisoned")
            .push(window);
    }

    pub fn window_count(&self) -> usize {
        self.windows.lock().expect("app state poisoned").len()
    }
}
