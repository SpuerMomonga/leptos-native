#![forbid(unsafe_code)]

use native::prelude::*;

fn counter() {}

fn main() {
    Application::default()
        .setup(|app| {
            let window = WindowBuilder::new()
                .id("counter")
                .title("Counter")
                .mount(counter)
                .build();

            app.set_window(window);
        })
        .run();
}
