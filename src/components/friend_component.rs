use super::models::Availability;
use leptos::*;

#[component]
pub fn Friend(
    availability: ReadSignal<Availability>,
    name: ReadSignal<String>,
    status: ReadSignal<String>,
    open_chat: impl Fn(usize) + 'static,
    order: Option<usize>,
) -> impl IntoView {
    let format_status = move || {
        if !status.get().is_empty() {
            format!(" - {}", status.get())
        } else {
            "".to_string()
        }
    };

    view! {
        <div id="friend_container" class="flex-row minline-20">
            <span>{move || availability.get().to_icon()}</span>
            <a on:click=move |ev| {
                ev.prevent_default();
                if order.is_some() {
                    open_chat(order.unwrap())
                }
            }>
                <span class="bold">{move || name.get()}</span>
            </a>
            <span class="ml-04">" " {move || format_status}</span>
        </div>
    }
}
