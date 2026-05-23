#![forbid(unsafe_code)]

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Runtime {
    started: bool,
}

impl Runtime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&mut self) {
        self.started = true;
    }

    pub fn is_started(&self) -> bool {
        self.started
    }
}
