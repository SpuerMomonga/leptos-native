#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signal<T> {
    value: T,
}

impl<T> Signal<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn set(&mut self, value: T) {
        self.value = value;
    }
}

pub fn signal<T>(value: T) -> Signal<T> {
    Signal::new(value)
}
