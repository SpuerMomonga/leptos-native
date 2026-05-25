// use leptos_native::prelude::*;

// #[component]
// fn Counter() -> impl IntoView {
//     let (count, set_count) = signal(0);
//     let doubled = memo(move |_| count.get() * 2);

//     effect(move |_| {
//         tracing::info!("main window count = {}", count.get());
//     });

//     view! {
//         <section class="counter">
//             <button on:click=move |_| set_count.update(|value| *value += 1)>
//                 "Increment"
//             </button>
//             <button on:click=move |_| set_count.update(|value| *value -= 1)>
//                 "Decrement"
//             </button>
//             <p>"Count: " {move || count.get()}</p>
//             <p>"Doubled: " {move || doubled.get()}</p>
//         </section>
//     }
// }

// #[component]
// fn Tools() -> impl IntoView {
//     view! {
//         <section class="tools">
//             <p>"Tools window"</p>
//         </section>
//     }
// }

// fn main() {
//     Application::default()
//         .setup(|app| {
//             let main_window = WindowBuilder::new("main", Counter)
//                 .title("leptos-native")
//                 .inner_size(960, 640)
//                 .resizable(true)
//                 .build();
//             app.open_window(main_window);

//             let tools_window = Window::builder("tools", Tools)
//                 .title("Tools")
//                 .inner_size(360, 520)
//                 .resizable(false)
//                 .build();
//             app.open_window(tools_window);
//         })
//         .run();
// }

fn main() {
    
}