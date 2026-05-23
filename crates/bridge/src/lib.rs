#![forbid(unsafe_code)]

use ipc::Message;

#[derive(Debug, Default)]
pub struct Bridge {
    messages: Vec<Message>,
}

impl Bridge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}
