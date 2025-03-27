use leptos::*;
use super::models;
use models::User;

#[component]
pub fn Message(user: ReadSignal<User>, content: ReadSignal<String>) -> impl IntoView {
    view! {
        <div class="message_container">
            <div class="message_content">
                <div class="message_author">{move || user.get().name}" says:"</div>
                <div class="message_text">{content.get()}</div>
            </div>
        </div>
    }
}