#![forbid(unsafe_code)]

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Window {
    id: Option<String>,
    title: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WindowBuilder {
    id: Option<String>,
    title: Option<String>,
}

impl Window {
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
}

impl WindowBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn mount<T>(self, _component: T) -> Self {
        self
    }

    pub fn build(self) -> Window {
        Window {
            id: self.id,
            title: self.title,
        }
    }
}
