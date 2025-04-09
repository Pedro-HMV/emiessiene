use super::models::Availability;
use leptos::prelude::*;

#[component]
pub fn Friend(
    availability: ReadSignal<Availability>,
    name: ReadSignal<String>,
    status: ReadSignal<String>,
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
                <span class="bold">{move || name.get()}</span>
            <span class="ml-04">" " {move || format_status}</span>
        </div>
    }
}
