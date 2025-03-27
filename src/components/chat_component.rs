use leptos::*;
use leptos::ev::SubmitEvent;
use leptos::web_sys;
use leptos::ev::KeyboardEvent;
use wasm_bindgen::JsCast;


use models::User;
use models::Friend;
use crate::components::message_component::Message;

use super::models;

#[component]
pub fn Chat(
    show: WriteSignal<bool>,
    user: ReadSignal<User>,
    friends: ReadSignal<(Vec<Friend>, Vec<Friend>)>,
    friend: ReadSignal<usize>,
    close: impl Fn(usize) + 'static,
) -> impl IntoView {
    let (msg, set_msg) = create_signal(String::new());
    let (message_list, set_message_list) = create_signal(Vec::new());
    let update_msg = move |ev| {
        let m = event_target_value(&ev);
        set_msg.set(m);
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

    view! {
        <main class="container">
            <div class="chat_container">
                <div class="chat_receiver-bar">
                    <button
                        class="back-button"
                        on:click=move |_| {
                            show.set(false);
                        }
                    >
                        { "⬅️" }
                    </button>
                    <span class="chat_receiver">{ "👤" } {&friends.get().0[friend.get()].name}</span>
                    <span class="chat_receiver-status-message">
                        {&friends.get().0[friend.get()].status}
                        <span class="ml-1">{format!("<{}>", &friends.get().0[friend.get()].email)}</span>
                    </span>
                    <button
                        class="close-button"
                        on:click=move |_| {
                            close(friend.get());
                        }
                    >
                        { "❌" }
                    </button>
                </div>
                <div class="chat_top-bar chat_icon-bar main_bordered">
                    <div class="chat_config-btn">{"⚙️"}</div>
                    <div class="chat_invite-btn">{"👤"}</div>
                    <div class="chat_files-btn">{"📁"}</div>
                    <div class="chat_webcam-btn">{"📷"}</div>
                    <div class="chat_voice-btn">{"📞"}</div>
                    <div class="chat_block-btn">{"🚫"}</div>
                </div>
                <div class="chat-and-avatars flex-row">
                    <div class="left-column">
                        <div class="chat_window">
                            <div class="chat_message-list main_bordered">
                                {move || {
                                    message_list
                                        .get()
                                        .iter()
                                        .map(|m| {
                                            view! {
                                                <Message user=user content=create_signal(m.clone()).0 />
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                }}
                            </div>
                        </div>
                        <div class="chat_mid-bar chat_icon-bar main_bordered">
                            <div class="chat_font-btn">{"🔤"}</div>
                            <div class="chat_emote-btn">{"😊"}</div>
                            <div class="chat_audio-btn">{"📢"}</div>
                            <div class="chat_image-btn">{"🖼️"}</div>
                            <div class="chat_nudge-btn">{"😵‍💫"}</div>
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
                                <button type="submit">{ "Send" }</button>
                            </form>
                        </div>
                    </div>
                    <div class="right-column flex-col">
                        <div class="top-user flex-col user-block">
                            <div class="top-avatar avatar"></div>
                            <div class="below-avatar flex-row justify-between">
                                <div class="webcam-icon">{"🎦"}</div>
                                <div class="options-arrow">{"🔽"}</div>
                            </div>
                        </div>
                        <div class="bottom-user flex-col user-block">
                            <div class="bottom-avatar avatar"></div>
                            <div class="below-avatar flex-row justify-between">
                                <div class="webcam-icon">{"🎦"}</div>
                                <div class="options-arrow">{"🔽"}</div>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </main>
    }
}