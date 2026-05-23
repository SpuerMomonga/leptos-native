#![forbid(unsafe_code)]

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrayIcon {
    tooltip: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct TrayIconBuilder {
    tooltip: Option<String>,
}

impl TrayIconBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn build(self) -> TrayIcon {
        TrayIcon {
            tooltip: self.tooltip,
        }
    }
}
