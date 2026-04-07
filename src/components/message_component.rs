use leptos::prelude::*;

#[component]
pub fn Message(author: String, body: String, pending: bool) -> impl IntoView {
    let class = if pending {
        "message_container message--pending"
    } else {
        "message_container"
    };
    view! {
        <div class=class>
            <div class="message_content">
                <div class="message_author">{author}" says:"</div>
                <div class="message_text">{body}</div>
            </div>
        </div>
    }
}
