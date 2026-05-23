#![forbid(unsafe_code)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    name: String,
    payload: String,
}

impl Message {
    pub fn new(name: impl Into<String>, payload: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            payload: payload.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn payload(&self) -> &str {
        &self.payload
    }
}
