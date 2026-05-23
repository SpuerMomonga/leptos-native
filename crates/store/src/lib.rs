#![forbid(unsafe_code)]

use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Store {
    values: HashMap<String, String>,
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }
}
