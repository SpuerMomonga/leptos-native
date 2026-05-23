#![forbid(unsafe_code)]

/// Framework name used by examples and future runtime code.
pub const FRAMEWORK_NAME: &str = "leptos-native";

pub fn framework_name() -> &'static str {
    FRAMEWORK_NAME
}
