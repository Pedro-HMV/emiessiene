use crate::app::invoke;
use leptos::ev::{FocusEvent, KeyboardEvent};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::web_sys::HtmlInputElement;
use leptos_router::components::A;
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use super::friend_component::Friend;
use super::models;
use models::{Friend, UpdateUsernameArgs, User};

#[component]
pub fn MainPage() -> impl IntoView {
    let (editing_user, set_editing_user) = signal(false);
    let (connection_status, set_connection_status) = signal("🔌 Checking connection...".to_string());

    // Use the global user context instead of creating a local one
    let user = use_context::<ReadSignal<User>>().expect("No user context");
    let set_user = use_context::<WriteSignal<User>>().expect("No user setter context");

    let friends =
        use_context::<ReadSignal<(Vec<Friend>, Vec<Friend>)>>().expect("No friends context");

    let open_chats =
        move || use_context::<ReadSignal<Vec<usize>>>().expect("No open chats context");

    let set_open_chats =
        use_context::<WriteSignal<Vec<usize>>>().expect("No set open chats context");

    // Check XMPP connection status on page load
    {
        let set_connection_status = set_connection_status;
        spawn_local(async move {
            match invoke("xmpp_get_connection_status", to_value(&serde_json::json!({})).unwrap()).await {
                result => {
                    match from_value::<serde_json::Value>(result) {
                        Ok(status) => {
                            if status["connected"].as_bool().unwrap_or(false) {
                                set_connection_status.set("🟢 Connected".to_string());
                            } else {
                                set_connection_status.set("� Disconnected".to_string());
                            }
                        }
                        Err(_) => {
                            set_connection_status.set("🔴 Disconnected".to_string());
                        }
                    }
                }
            }
        });
    }

    // Set up XMPP event listeners
    {
        let set_connection_status = set_connection_status;
        spawn_local(async move {
            // Import Tauri's event listening capability
            #[wasm_bindgen]
            extern "C" {
                #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
                async fn listen(event: &str, callback: &js_sys::Function) -> JsValue;
            }

            // Listen for connection events
            let connected_callback = wasm_bindgen::closure::Closure::wrap(Box::new({
                let set_connection_status = set_connection_status;
                move |_event: JsValue| {
                    set_connection_status.set("🟢 Connected".to_string());
                }
            }) as Box<dyn Fn(JsValue)>);

            let disconnected_callback = wasm_bindgen::closure::Closure::wrap(Box::new({
                let set_connection_status = set_connection_status;
                move |_event: JsValue| {
                    set_connection_status.set("🔴 Disconnected".to_string());
                }
            }) as Box<dyn Fn(JsValue)>);

            let _ = listen("xmpp_connected", connected_callback.as_ref().unchecked_ref()).await;
            let _ = listen("xmpp_disconnected", disconnected_callback.as_ref().unchecked_ref()).await;
            
            connected_callback.forget();
            disconnected_callback.forget();
        });
    }

    let online_friends = move || friends.get().0;
    let offline_friends = move || friends.get().1;

    // Function to close a chat tab
    let close_chat = move |chat_id: usize| {
        set_open_chats.update(|chats| {
            chats.retain(|&id| id != chat_id);
        });
    };

    let update_username = {
        move |ev: FocusEvent| {
            ev.prevent_default();
            let set_user = set_user.clone();
            spawn_local(async move {
                let username = event_target_value(&ev);
                let result = invoke(
                    "update_username",
                    to_value(&UpdateUsernameArgs { name: &username }).unwrap(),
                )
                .await;
                let updated_user: User = from_value(result).expect("Failed to parse user info");
                set_user.update(|user| {
                    user.name = updated_user.name;
                });
            });
            set_editing_user.set(false);
        }
    };

    let blur_on_enter = {
        move |ev: KeyboardEvent| {
            if ev.key() == "Enter" {
                let target = ev.target().unwrap();
                let el: &HtmlInputElement = target.dyn_ref().expect("Failed to get input element");
                el.blur().unwrap();
            }
        }
    };

    let chat_tabs = move || {
        open_chats()
            .get()
            .iter()
            .map(|&id| {
                let friend = online_friends()[id].clone();
                let close_chat = close_chat.clone();
                view! {
                    <div class="chat-tab-container">
                        <A href=move || { format!("/chat/{}", id) }>
                            <button class="chat-tab">{friend.name}</button>
                        </A>
                        <button class="chat-tab-close" on:click=move |_| close_chat(id)>
                            "❌"
                        </button>
                    </div>
                }
            })
            .collect::<Vec<_>>()
    };

    view! {
        <div id="main-container" class="flex-col">
            <div class="chat-tabs">{chat_tabs}</div>
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
                                <Show
                                    when=move || { editing_user.get() }
                                    fallback=move || {
                                        view! {
                                            <span
                                                on:click=move |_| set_editing_user.set(true)
                                                class="bold"
                                            >
                                                {move || user.get().name}
                                            </span>
                                        }
                                    }
                                >
                                    <input
                                        on:blur=update_username
                                        on:keydown=blur_on_enter
                                        class="user-edit_input"
                                        value=move || user.get().name
                                    />
                                </Show>
                                " ("
                                {move || user.get().availability.to_string()}
                                ")"
                                <span class="tabbed-down-arrow">"🔽"</span>
                            </div>
                            <div id="status-message">
                                {move || user.get().status}
                                <span class="tabbed-down-arrow">"🔽"</span>
                            </div>
                            <div id="connection-status" class="mt-1">
                                <span class="mr-1">
                                    {move || {
                                        if connection_status.get() == "Connected" {
                                            "🟢"
                                        } else {
                                            "🔴"
                                        }
                                    }}
                                </span>
                                <span style="font-size: 12px; color: #666;">
                                    {move || format!("XMPP: {}", connection_status.get())}
                                </span>
                            </div>
                            <A href="/">"Sign Out"</A>
                        </div>
                    </div>
                </div>
            </header>
            <div id="find-friends" class="mt-1 mb-1 bg-white border-1b pd-block-5 pd-inline-2">
                <span class="mr-1">"👤"</span>
                <input
                    type="text"
                    id="find-friend_input"
                    placeholder="Find a friend"
                    class="border-1b"
                />
                <span>"➕"</span>
            </div>
            <div id="friends-container" class="flex-col flex-grow p-10 bg-white auto-y">
                <span class="bold">"🔽 Friends"</span>
                <ul id="online-list">
                    <For
                        each=move || {
                            online_friends().clone().into_iter().enumerate().collect::<Vec<_>>()
                        }
                        key=|f| f.0
                        children=move |(id, friend)| {
                            let friend = friend.clone();
                            view! {
                                <li>
                                    <A href=move || format!("/chat/{id}", id = id)>
                                        <Friend
                                            availability=signal(friend.availability).0
                                            name=signal(friend.name).0
                                            status=signal(friend.status).0
                                        />
                                    </A>
                                </li>
                            }
                        }
                    />
                </ul>
                <span class="mt-1 bold">"🔽 Offline"</span>
                <ul id="offline-list">
                    <For
                        each=move || {
                            offline_friends().clone().into_iter().enumerate().collect::<Vec<_>>()
                        }
                        key=|f| f.0
                        children=move |(_, friend)| {
                            let friend = friend.clone();
                            view! {
                                <li>
                                    <Friend
                                        availability=signal(friend.availability).0
                                        name=signal(friend.name).0
                                        status=signal(friend.status).0
                                    />
                                </li>
                            }
                        }
                    />
                </ul>
            </div>
        </div>
    }
}
