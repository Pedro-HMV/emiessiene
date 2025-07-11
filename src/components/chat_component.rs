use leptos::ev::KeyboardEvent;
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::view;
use leptos::web_sys;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map};
use wasm_bindgen::JsCast;

use models::Friend;

use super::message_component::Message;
use super::models;

#[component]
pub fn Chat(// show: WriteSignal<bool>,
    // user: ReadSignal<User>,
    // friends: ReadSignal<(Vec<Friend>, Vec<Friend>)>,
    // friend: ReadSignal<usize>,
    // close: impl Fn(usize) + 'static,
) -> impl IntoView {
    let (msg, set_msg) = signal(String::new());
    let (message_list, set_message_list) = signal(Vec::new());
    let update_msg = move |ev| {
        let m = event_target_value(&ev);
        set_msg.set(m);
    };

    let set_open_chats = use_context::<WriteSignal<Vec<usize>>>();

    let friends =
        use_context::<ReadSignal<(Vec<Friend>, Vec<Friend>)>>().expect("No friends context");

    let navigate = use_navigate();

    let params = move || use_params_map();

    // Capture the friend_id once at component initialization
    let static_friend_id = params()
        .read()
        .get("id")
        .and_then(|id| id.parse::<usize>().ok())
        .unwrap_or(0);

    let friend_id = move || static_friend_id;

    // Function to close this chat - simple synchronous approach
    let close_chat = move |ev: leptos::ev::MouseEvent| {
        ev.prevent_default();
        ev.stop_propagation();

        log::info!(
            "Close chat button clicked for friend_id: {}",
            static_friend_id
        );

        // Update the open chats immediately - only if context is available
        if let Some(set_open_chats) = set_open_chats {
            set_open_chats.update(|chats| {
                log::info!("Before update - open chats: {:?}", chats);
                chats.retain(|&id| id != static_friend_id);
                log::info!("After update - open chats: {:?}", chats);
            });
        }

        // Navigate immediately
        log::info!("Navigating to /main");
        navigate("/main", Default::default());
    };

    let submit_on_enter = move |ev: KeyboardEvent| {
        spawn_local(async move {
            if ev.key() == "Enter" && !ev.shift_key() {
                ev.prevent_default();
                if let Some(form) = document().get_element_by_id("message-form") {
                    if let Some(element) = form.dyn_ref::<web_sys::HtmlFormElement>() {
                        element.request_submit().unwrap();
                    }
                }
            }
        })
    };

    let send_msg = move |ev: SubmitEvent| {
        ev.prevent_default();
        spawn_local(async move {
            let msg = msg.get_untracked();
            if msg.trim().is_empty() {
                return;
            }
            set_message_list.update(|msg_list| msg_list.push(msg.clone()));
            set_msg.set(String::new());
            if let Some(input) = document().get_element_by_id("message-input") {
                if let Some(input_element) = input.dyn_ref::<web_sys::HtmlTextAreaElement>() {
                    input_element.set_value("");
                }
            }
        });
    };

    // Add this chat to open chats if context is available
    if let Some(set_open_chats) = set_open_chats {
        set_open_chats.update(|chats| {
            if !chats.contains(&static_friend_id) {
                chats.push(static_friend_id);
            }
        });
    }

    view! {
        <main class="container">
            <div class="chat_container">
                <div class="chat_receiver-bar">
                    <div class="chat-header-left">
                        <A href="/main">
                            <button class="back-button" title="Back to Main">
                                "⬅️"
                            </button>
                        </A>
                    </div>

                    <div class="chat-header-center">
                        <span class="chat_receiver">
                            {move || {
                                format!(
                                    "👤 {status}",
                                    status = { friends.get().0[friend_id()].name.clone() },
                                )
                            }}
                        </span>
                        <span class="chat_receiver-status-message">
                            {move || friends.get().0[friend_id()].status.clone()}
                            <span class="ml-1">
                                {move || {
                                    format!("<{}>", friends.get().0[friend_id()].email.clone())
                                }}
                            </span>
                        </span>
                    </div>

                    <div class="chat-header-right">
                        <button class="close-button" on:click=close_chat title="Close Chat">
                            "❌"
                        </button>
                    </div>
                </div>
                <div class="chat_top-bar chat_icon-bar main_bordered">
                    <div class="chat_config-btn">"⚙️"</div>
                    <div class="chat_invite-btn">"👤"</div>
                    <div class="chat_files-btn">"📁"</div>
                    <div class="chat_webcam-btn">"📷"</div>
                    <div class="chat_voice-btn">"📞"</div>
                    <div class="chat_block-btn">"🚫"</div>
                </div>
                <div class="flex-row chat-and-avatars">
                    <div class="left-column">
                        <div class="chat_window">
                            <div class="chat_message-list main_bordered">
                                {move || {
                                    message_list
                                        .get()
                                        .iter()
                                        .map(|m| {
                                            view! { <Message content=signal(m.clone()).0 /> }
                                        })
                                        .collect::<Vec<_>>()
                                }}
                            </div>
                        </div>
                        <div class="chat_mid-bar chat_icon-bar main_bordered">
                            <div class="chat_font-btn">"🔤"</div>
                            <div class="chat_emote-btn">"😊"</div>
                            <div class="chat_audio-btn">"📢"</div>
                            <div class="chat_image-btn">"🖼️"</div>
                            <div class="chat_nudge-btn">"😵‍💫"</div>
                        </div>
                        <div class="chat_message-input">
                            <form id="message-form" class="chat_message-form" on:submit=send_msg>
                                <textarea
                                    class="main_bordered"
                                    id="message-input"
                                    placeholder="Enter a message..."
                                    on:input=update_msg
                                    on:keypress=submit_on_enter
                                ></textarea>
                                <button type="submit">"Send"</button>
                            </form>
                        </div>
                    </div>
                    <div class="flex-col right-column">
                        <div class="flex-col top-user user-block">
                            <div class="top-avatar avatar"></div>
                            <div class="flex-row justify-between below-avatar">
                                <div class="webcam-icon">"🎦"</div>
                                <div class="options-arrow">"🔽"</div>
                            </div>
                        </div>
                        <div class="flex-col bottom-user user-block">
                            <div class="bottom-avatar avatar"></div>
                            <div class="flex-row justify-between below-avatar">
                                <div class="webcam-icon">"🎦"</div>
                                <div class="options-arrow">"🔽"</div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </main>
    }
}
