use std::collections::HashMap;

use ev::KeyboardEvent;
use leptos::leptos_dom::ev::SubmitEvent;
use leptos::*;
use leptos_router::{Route, Router, Routes, A};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use tracing::event;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "tauri"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

const ONLINE_FRIENDS: &[(Availability, &str, &str, &str)] = &[
    (Availability::Online, "Pedro", "Far far away, behind the word", "pedro@hotmail.com"),
    (Availability::Online, "John", "One day", "john@hotmail.com"),
    (Availability::Online, "Jane", "Duden flows by their place and supplies it with the necessary regelialia", "jane@hotmail.com"),
    (Availability::Busy, "Bob", "mountains, far from the countries", "bob@hotmail.com"),
    (Availability::Busy, "Alice", "a small line of blind text", "alice@hotmail.com"),
    (Availability::Away, "Charlie", "if she hasn't been rewritten, then they are still using her", "charlie@hotmail.com"),
    (Availability::Online, "David", "The Big Oxmox", "david@hotmail.com"),
    (Availability::Online, "Eve", "A wonderful serenity has taken possession of my entire soul", "eve@hotmail.com"),
    (Availability::Away, "Frank", "I hear the buzz of the little world among the stalks", "frank@hotmail.com"),
    (Availability::Busy, "Grace", "O my friend -- but it is too much for my strength -- I sink under the weight of the splendour of these visions!", "grace@hotmail.com"),
    (Availability::Online, "Henry", "the lovely valley teems with vapour around me, and the meridian sun strikes the upper surface of the impenetrable foliage of my trees", "henry@hotmail.com"),
    (Availability::Online, "Anderson", "Rei do front", "anderson@hotmail.com"),
];

const OFFLINE_FRIENDS: &[(Availability, &str, &str, &str)] = &[
    (Availability::Offline, "Liam", "", "liam@hotmail.com"),
    (Availability::Offline, "Emma", "", "emma@hotmail.com"),
    (Availability::Offline, "Noah", "", "noah@hotmail.com"),
    (Availability::Offline, "Olivia", "", "olivia@hotmail.com"),
    (Availability::Offline, "Ethan", "", "ethan@hotmail.com"),
    (Availability::Offline, "Sophia", "", "sophia@hotmail.com"),
    (Availability::Offline, "Mason", "", "mason@hotmail.com"),
    (Availability::Offline, "Ava", "", "ava@hotmail.com"),
    (Availability::Offline, "William", "", "william@hotmail.com"),
    (
        Availability::Offline,
        "Isabella",
        "",
        "isabella@hotmail.com",
    ),
    (Availability::Offline, "James", "", "james@hotmail.com"),
    (
        Availability::Offline,
        "Charlotte",
        "",
        "charlotte@hotmail.com",
    ),
    (
        Availability::Offline,
        "Benjamin",
        "",
        "benjamin@hotmail.com",
    ),
    (Availability::Offline, "Amelia", "", "amelia@hotmail.com"),
    (Availability::Offline, "Lucas", "", "lucas@hotmail.com"),
    (Availability::Offline, "Mia", "", "mia@hotmail.com"),
    (Availability::Offline, "Henry", "", "henry@hotmail.com"),
    (Availability::Offline, "Harper", "", "harper@hotmail.com"),
    (
        Availability::Offline,
        "Alexander",
        "",
        "alexander@hotmail.com",
    ),
    (Availability::Offline, "Evelyn", "", "evelyn@hotmail.com"),
    (Availability::Offline, "Daniel", "", "daniel@hotmail.com"),
    (Availability::Offline, "The Last", "", "thelast@hotmail.com"),
];
#[derive(Clone)]
pub enum Availability {
    Online,
    Away,
    Busy,
    Offline,
}

impl Availability {
    fn to_icon(&self) -> String {
        match self {
            Availability::Online => "👤".to_string(),
            Availability::Away => "⏳".to_string(),
            Availability::Busy => "⛔".to_string(),
            Availability::Offline => "📴".to_string(),
        }
    }
}

#[component]
pub fn Friend(
    availability: ReadSignal<Availability>,
    name: ReadSignal<String>,
    status: ReadSignal<String>,
    email: ReadSignal<String>,
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
            <span>{availability.get().to_icon()}</span>
            <a on:click=move |ev| {
                ev.prevent_default();
                if order.is_some() {
                    open_chat(order.unwrap())
                }
            }>
                <span class="bold">{name.get()}</span>
            </a>
            {format_status}
        </div>
    }
}

#[component]
pub fn MainPage() -> impl IntoView {
    let (show_chat, set_show_chat) = create_signal(false);
    let (friend_id, set_friend_id) = create_signal(0);
    let (open_chats, set_open_chats) = create_signal(Vec::new());

    let open_new_chat = move |id: usize| {
        set_open_chats.update(|chats| {
            if !chats.contains(&id) {
                chats.push(id);
            }
        });
        set_friend_id.set(id);
        set_show_chat.set(true);
    };

    let close_chat = move |id: usize| {
        set_open_chats.update(|chats| {
            chats.retain(|&x| x != id);
        });

        set_show_chat.set(false);
    };

    view! {
        <div id="main-container" class="flex-col">
            <div class="chat-tabs">
                {move || {
                    open_chats
                        .get()
                        .iter()
                        .map(|&id| {
                            let friend = &ONLINE_FRIENDS[id];
                            view! {
                                <button
                                    class="chat-tab"
                                    class:active=move || friend_id.get() == id
                                    on:click=move |_| {
                                        set_friend_id.set(id);
                                        set_show_chat.set(true);
                                    }
                                >
                                    {friend.1}
                                </button>
                            }
                        })
                        .collect::<Vec<_>>()
                }}
            </div>
            {move || {
                if show_chat.get() {
                    view! { <Chat show=set_show_chat friend=friend_id close=close_chat /> }
                } else {
                    view! {
                        <header>
                            <div id="header_container" class="flex-row p-10 border-st">
                                <div id="header_left">
                                    <div id="header_avatar">
                                        <div
                                            id="avatar_img"
                                            style="width: 90px; height: 90px; background: black;"
                                        >
                                            a
                                        </div>
                                    </div>
                                </div>
                                <div id="header_right" class="ml-1">
                                    <div id="header_info">
                                        <div id="name">
                                            <span class="bold">Pedro</span>
                                            " (Online)"
                                            <span class="tabbed-down-arrow">"🔽"</span>
                                        </div>
                                        <div id="status-message">
                                            "<Tá saindo da jaula o MSNtro!>"
                                            <span class="tabbed-down-arrow">"🔽"</span>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        </header>
                        <div
                            id="find-friends"
                            class="border-1b bg-white pd-block-5 pd-inline-2 mb-1 mt-1"
                        >
                            <span class="mr-1">"👤"</span>
                            <input
                                type="text"
                                id="find-friend_input"
                                placeholder="Find a friend"
                                class="border-1b"
                            />
                            <span>"➕"</span>
                        </div>
                        <div id="friends-container" class="flex-col bg-white auto-y p-10 flex-grow">
                            <span class="bold">"🔽 Friends"</span>
                            <ul id="online-list">
                                {move || {
                                    ONLINE_FRIENDS
                                        .iter()
                                        .enumerate()
                                        .map(|(i, f)| {
                                            view! {
                                                <li>
                                                    <Friend
                                                        availability=create_signal(f.0.clone()).0
                                                        name=create_signal(f.1.to_string()).0
                                                        status=create_signal(f.2.to_string()).0
                                                        email=create_signal(f.3.to_string()).0
                                                        open_chat=open_new_chat
                                                        order=Some(i)
                                                    />
                                                </li>
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                }}
                            </ul>
                            <span class="bold mt-1">"🔽 Offline"</span>
                            <ul id="offline-list">
                                {move || {
                                    OFFLINE_FRIENDS
                                        .iter()
                                        .map(|f| {
                                            view! {
                                                <li>
                                                    <Friend
                                                        availability=create_signal(f.0.clone()).0
                                                        name=create_signal(f.1.to_string()).0
                                                        status=create_signal(f.2.to_string()).0
                                                        email=create_signal(f.3.to_string()).0
                                                        open_chat=open_new_chat
                                                        order=None
                                                    />
                                                </li>
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                }}
                            </ul>
                        </div>
                    }
                        .into_view()
                }
            }}
        </div>
    }
}

#[component]
pub fn Message(content: ReadSignal<String>) -> impl IntoView {
    view! {
        <div class="message_container">
            <div class="message_content">
                <div class="message_author">"Pedro says:"</div>
                <div class="message_text">{content.get()}</div>
            </div>
        </div>
    }
}

#[component]
pub fn Chat(
    show: WriteSignal<bool>,
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
                        "⬅️"
                    </button>
                    <span class="chat_receiver">"👤" {ONLINE_FRIENDS[friend.get()].1}</span>
                    <span class="chat_receiver-status-message">
                        {ONLINE_FRIENDS[friend.get()].2}
                        <span class="ml-1">"<"{ONLINE_FRIENDS[friend.get()].3}">"</span>
                    </span>
                    <button
                        class="close-button"
                        on:click=move |_| {
                            close(friend.get());
                        }
                    >
                        "❌"
                    </button>
                </div>
                <div class="chat_top-bar chat_icon-bar main_bordered">
                    <div class="chat_config-btn">"⚙️"</div>
                    <div class="chat_invite-btn">"👤"</div>
                    <div class="chat_files-btn">"📁"</div>
                    <div class="chat_webcam-btn">"📷"</div>
                    <div class="chat_voice-btn">"📞"</div>
                    <div class="chat_block-btn">"🚫"</div>
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
                                            view! { <Message content=create_signal(m.clone()).0 /> }
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
                    <div class="right-column flex-col">
                        <div class="top-user flex-col user-block">
                            <div class="top-avatar avatar"></div>
                            <div class="below-avatar flex-row justify-between">
                                <div class="webcam-icon">"🎦"</div>
                                <div class="options-arrow">"🔽"</div>
                            </div>
                        </div>
                        <div class="bottom-user flex-col user-block">
                            <div class="bottom-avatar avatar"></div>
                            <div class="below-avatar flex-row justify-between">
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

#[component]
pub fn LoginPage() -> impl IntoView {
    let (availability, set_availability) = create_signal(Availability::Online);
    let (remember_me, set_remember_me) = create_signal(false);
    let (auto_sign_in, set_auto_sign_in) = create_signal(false);

    let update_availability = move |ev| {
        let value = event_target_value(&ev);
        set_availability.set(match value.as_str() {
            "Away" => Availability::Away,
            "Busy" => Availability::Busy,
            "Offline" => Availability::Offline,
            _ => Availability::Online,
        });
    };

    view! {
        <div id="login_container" class="flex-col">
            <div id="login_title">"EmiEssiEne"</div>
            <div id="login_avatar">
                <div id="login_avatar_img" style="width: 150px; height: 150px; background: black;">
                    a
                </div>
            </div>
            <form id="login_form" class="flex-col">
                <input type="text" id="login_username" placeholder="Username" />
                <input type="password" id="login_password" placeholder="Password" />
                <div>
                    "Status: "<select id="login_availability" on:change=update_availability>
                        <option value="Online">Online</option>
                        <option value="Busy">Busy</option>
                        <option value="Away">Away</option>
                        <option value="Offline">Offline</option>
                    </select>
                </div>
                <div class="checkbox-container">
                    <label>
                        <input
                            type="checkbox"
                            id="remember_me"
                            checked=remember_me
                            on:change=move |ev| set_remember_me.set(event_target_checked(&ev))
                        />
                        "Remember me"
                    </label>
                </div>
                <div class="checkbox-container">
                    <label>
                        <input
                            type="checkbox"
                            id="auto_sign_in"
                            checked=auto_sign_in
                            on:change=move |ev| set_auto_sign_in.set(event_target_checked(&ev))
                        />
                        "Sign me in automatically"
                    </label>
                </div>
                <button type="submit" on:click=sign_in>
                    "Sign In"
                </button>
            </form>
        </div>
    }
}
