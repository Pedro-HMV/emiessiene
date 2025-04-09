use super::models;
use leptos::prelude::ElementChild;
use leptos::prelude::*;
use models::User;

#[component]
pub fn Message(content: ReadSignal<String>) -> impl IntoView {
    let user = use_context::<ReadSignal<User>>().expect("No user context");
    view! {
        <div class="message_container">
            <div class="message_content">
                <div class="message_author">{move || user.get().name}" says:"</div>
                <div class="message_text">{content.get()}</div>
            </div>
        </div>
    }
}
