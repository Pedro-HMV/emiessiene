use leptos::ev::KeyboardEvent;
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::view;
use leptos::web_sys;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use models::Friend;

use super::message_component::Message;
use super::models;

// Import the app's invoke function
use crate::app::invoke;

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
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            if let Some(form) = document().get_element_by_id("message-form") {
                if let Some(element) = form.dyn_ref::<web_sys::HtmlFormElement>() {
                    let _ = element.request_submit();
                }
            }
        }
    };

    let send_msg = move |ev: SubmitEvent| {
        ev.prevent_default();
        spawn_local(async move {
            let msg = msg.get_untracked();
            if msg.trim().is_empty() {
                return;
            }

            // Get the friend's information for XMPP messaging
            let friend_info = friends.get();
            if friend_id() >= friend_info.0.len() {
                log::error!("Invalid friend index: {}", friend_id());
                return;
            }

            let friend = &friend_info.0[friend_id()];
            let friend_jid = friend.email.clone(); // Using email as JID

            // Send message via XMPP
            let send_args = serde_json::json!({
                "to_jid": friend_jid.clone(),
                "body": msg.trim().to_string(),
            });

            match invoke("xmpp_send_message", to_value(&send_args).unwrap()).await {
                result => {
                    match from_value::<serde_json::Value>(result) {
                        Ok(response) => {
                            if response["success"].as_bool().unwrap_or(false) {
                                log::info!("Message sent successfully via XMPP to {}", friend_jid);
                                // Add message to local list only after successful send
                                set_message_list.update(|msg_list| msg_list.push(msg.clone()));
                            } else {
                                log::error!("Failed to send XMPP message: {:?}", response);
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to parse XMPP response: {:?}", e);
                        }
                    }
                }
            }

            // Clear the input field
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

    // Set up XMPP event listeners for incoming messages
    {
        let set_message_list = set_message_list;

        spawn_local(async move {
            // Import Tauri's event listening capability
            #[wasm_bindgen]
            extern "C" {
                #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
                async fn listen(event: &str, callback: &js_sys::Function) -> JsValue;
            }

            let callback = wasm_bindgen::closure::Closure::wrap(Box::new(move |event: JsValue| {
                // Parse the incoming message event
                match from_value::<serde_json::Value>(event) {
                    Ok(event_data) => {
                        log::info!("Received XMPP event: {:?}", event_data);

                        // For incoming messages, add them to the message list
                        if let Some(from) = event_data["from"].as_str() {
                            if let Some(body) = event_data["body"].as_str() {
                                let incoming_message = format!("📥 {}: {}", from, body);
                                set_message_list.update(|msg_list| msg_list.push(incoming_message));
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to parse XMPP event: {:?}", e);
                    }
                }
            })
                as Box<dyn Fn(JsValue)>);

            let _ = listen("xmpp_message_received", callback.as_ref().unchecked_ref()).await;
            callback.forget(); // Keep the callback alive
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
                            {move || friends.get().0[friend_id()].flavour_text.clone()}
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
                                    on:keydown=submit_on_enter
                                ></textarea>
                                <button type="submit">"Send"</button>
                            </form>
                        </div>
                    </div>
                    <div class="flex-col right-column">
                        <div class="flex-col top-user user-block">
                            <div class="top-avatar avatar"></div>
                            <div class="below-avatar">
                                <div class="webcam-icon">"🎦"</div>
                                <div class="options-arrow">"🔽"</div>
                            </div>
                        </div>
                        <div class="flex-col bottom-user user-block">
                            <div class="bottom-avatar avatar"></div>
                            <div class="below-avatar">
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
