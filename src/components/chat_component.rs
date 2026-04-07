use leptos::ev::KeyboardEvent;
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::view;
use leptos::web_sys;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::JsCast;

use super::message_component::Message;
use super::models;
use models::{ChatMessage, Friend, MessageStore, User, XmppSendMessageArgs};

// Import the app's invoke function
use crate::app::invoke;

#[component]
pub fn Chat() -> impl IntoView {
    let (msg, set_msg) = signal(String::new());
    let update_msg = move |ev| set_msg.set(event_target_value(&ev));

    let set_open_chats = use_context::<WriteSignal<Vec<String>>>();
    let friends =
        use_context::<ReadSignal<(Vec<Friend>, Vec<Friend>)>>().expect("No friends context");
    let user = use_context::<ReadSignal<User>>().expect("No user context");
    // Global message store provided by App
    let messages = use_context::<RwSignal<MessageStore>>().expect("No messages context");

    let navigate = use_navigate();

    // Read the bare JID from the route parameter (e.g. /chat/alice@localhost)
    let jid_param = use_params_map()
        .get_untracked()
        .get("id")
        .unwrap_or_default();

    // Look up the friend by JID. Returns a clone each call — two separate closures used in view.
    let friend_info_name = {
        let jid = jid_param.clone();
        move || {
            let (online, offline) = friends.get();
            online
                .into_iter()
                .chain(offline)
                .find(|f| f.email == jid)
                .map(|f| f.name)
                .unwrap_or_else(|| jid.clone())
        }
    };
    let friend_info_status = {
        let jid = jid_param.clone();
        move || {
            let (online, offline) = friends.get();
            online
                .into_iter()
                .chain(offline)
                .find(|f| f.email == jid)
                .map(|f| format!("{} <{}>", f.flavour_text, f.email))
                .unwrap_or_default()
        }
    };

    // Close this chat and return to main page
    let close_chat = {
        let jid = jid_param.clone();
        move |ev: leptos::ev::MouseEvent| {
            ev.prevent_default();
            ev.stop_propagation();
            if let Some(set_open_chats) = set_open_chats {
                let jid = jid.clone();
                set_open_chats.update(|chats| chats.retain(|j| j != &jid));
            }
            navigate("/main", Default::default());
        }
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

    let send_msg = {
        let jid = jid_param.clone();
        move |ev: SubmitEvent| {
            ev.prevent_default();
            let jid = jid.clone();
            let messages = messages;
            spawn_local(async move {
                let body = msg.get_untracked();
                if body.trim().is_empty() {
                    return;
                }
                let body = body.trim().to_string();

                // Optimistically append to global store with pending=true (not yet confirmed by server)
                let self_jid = user.get_untracked().email;
                let optimistic_msg = ChatMessage {
                    from_jid: self_jid,
                    body: body.clone(),
                    timestamp: String::new(),
                    is_self: true,
                    pending: true,
                };
                let mut pushed_idx: usize = 0;
                messages.update(|store| {
                    let conv = store.entry(jid.clone()).or_default();
                    conv.push(optimistic_msg);
                    if conv.len() > 200 {
                        conv.drain(0..conv.len() - 200);
                    }
                    pushed_idx = conv.len() - 1;
                });

                set_msg.set(String::new());
                if let Some(input) = document().get_element_by_id("message-input") {
                    if let Some(el) = input.dyn_ref::<web_sys::HtmlTextAreaElement>() {
                        el.set_value("");
                    }
                }

                // Send via XMPP
                let args = XmppSendMessageArgs {
                    to_jid: jid.clone(),
                    body: body.clone(),
                };
                let result = invoke("xmpp_send_message", to_value(&args).unwrap()).await;
                match from_value::<serde_json::Value>(result) {
                    Ok(response) if response["success"].as_bool().unwrap_or(false) => {
                        // Confirm delivery — clear the pending flag
                        messages.update(|store| {
                            if let Some(conv) = store.get_mut(&jid) {
                                if let Some(msg) = conv.get_mut(pushed_idx) {
                                    msg.pending = false;
                                }
                            }
                        });
                        leptos::logging::log!("Message sent to {}", jid);
                    }
                    Ok(response) => {
                        leptos::logging::warn!("xmpp_send_message error: {:?}", response["error"]);
                    }
                    Err(e) => {
                        leptos::logging::warn!("xmpp_send_message parse error: {:?}", e);
                    }
                }
            });
        }
    };

    // Register this JID in open_chats so the main page tab tracks it
    if let Some(set_open_chats) = set_open_chats {
        let jid = jid_param.clone();
        set_open_chats.update(|chats| {
            if !chats.contains(&jid) {
                chats.push(jid);
            }
        });
    }

    // Derive the displayed message list reactively from the global store
    let displayed_messages = {
        let jid = jid_param.clone();
        move || -> Vec<ChatMessage> { messages.get().get(&jid).cloned().unwrap_or_default() }
    };
    // Clone for the author lookup inside the message rendering closure
    let jid_for_msgs = jid_param.clone();

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
                            {move || format!("\u{1F464} {}", friend_info_name())}
                        </span>
                        <span class="chat_receiver-status-message">
                            {move || friend_info_status()}
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
                                    let user_name = user.get().name;
                                    let (online, offline) = friends.get();
                                    let friend_name = online
                                        .into_iter()
                                        .chain(offline)
                                        .find(|f| f.email == jid_for_msgs)
                                        .map(|f| f.name)
                                        .unwrap_or_else(|| {
                                            jid_for_msgs
                                                .split('@')
                                                .next()
                                                .unwrap_or(&jid_for_msgs)
                                                .to_string()
                                        });
                                    displayed_messages()
                                        .into_iter()
                                        .map(|m| {
                                            let author = if m.is_self {
                                                user_name.clone()
                                            } else {
                                                friend_name.clone()
                                            };
                                            view! {
                                                <Message author=author body=m.body pending=m.pending />
                                            }
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
