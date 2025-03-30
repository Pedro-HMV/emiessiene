use crate::app::invoke;
use leptos::ev::{FocusEvent, KeyboardEvent};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::web_sys::HtmlInputElement;
use leptos_router::components::A;
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::{JsCast, JsValue};

use super::models;
use super::{chat_component::Chat, friend_component::Friend};
use models::{Availability, Friend, UpdateUsernameArgs, User};

#[component]
pub fn MainPage() -> impl IntoView {
    let (show_chat, set_show_chat) = signal(false);
    let (friend_id, set_friend_id) = signal(0);
    let (open_chats, set_open_chats) = signal(Vec::new());
    let (editing_user, set_editing_user) = signal(false);
    let (user, set_user) = signal(User {
        name: "Username".to_string(),
        email: "user@hotmail.com".to_string(),
        status: "Status message".to_string(),
        availability: Availability::Online,
    });
    let (friends, set_friends) = signal((Vec::new(), Vec::new()));

    let user_resource = LocalResource::new(|| async {
        let info = invoke("get_user", JsValue::null()).await;
        from_value::<User>(info).expect("Failed to parse user info")
    });

    let friends_resource = LocalResource::new(|| async {
        let info = invoke("get_friends", JsValue::null()).await;
        from_value::<(Vec<Friend>, Vec<Friend>)>(info).expect("Failed to parse friends info")
    });

    Effect::new(move |_| {
        if let Some(updated_user) = user_resource.get() {
            set_user.set((*updated_user).clone());
        }
        if let Some(updated_friends) = friends_resource.get() {
            set_friends.set((*updated_friends).clone());
        }
    });

    // Trigger initial loading
    user_resource.refetch();
    friends_resource.refetch();

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

    let chat_tabs = move || {
        open_chats
            .get()
            .iter()
            .map(|&id| {
                let friend = &friends.get().0[id];
                view! {
                    <button
                        class="chat-tab"
                        class:active=move || friend_id.get() == id
                        on:click=move |_| {
                            set_friend_id.set(id);
                            set_show_chat.set(true);
                        }
                    >
                        {friend.name.clone()}
                    </button>
                }
            })
            .collect::<Vec<_>>()
    };

    let user = user.clone();
    let set_editing_user = set_editing_user.clone();

    view! {
        <div id="main-container" class="flex-col">
            <div class="chat-tabs">
                {chat_tabs}
            </div>
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
                                <Show   when=move || {editing_user.get()}
                                        fallback=move || view! {
                                            <span
                                                on:click=move |_| set_editing_user.set(true)
                                                class="bold"
                                            >
                                                {move || user.get().name}
                                            </span>}
                                >
                                    <input
                                        on:blur=update_username
                                        on:keydown=blur_on_enter
                                        class="user-edit_input"
                                        value=move || user.get().name
                                    />
                                </Show>
                                " (" {move || user.get().availability.to_string()} ")"
                                <span class="tabbed-down-arrow">"🔽"</span>
                            </div>
                            <div id="status-message">
                                {move || user.get().status}
                                <span class="tabbed-down-arrow">"🔽"</span>
                            </div>
                            <A href="/">
                                "Sign Out"
                            </A>
                        </div>
                    </div>
                </div>
            </header>
            <div
                id="find-friends"
                class="mt-1 mb-1 bg-white border-1b pd-block-5 pd-inline-2"
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
            <div id="friends-container" class="flex-col flex-grow p-10 bg-white auto-y">
                <span class="bold">"🔽 Friends"</span>
                <ul id="online-list">
                    {move || {
                        friends
                            .get()
                            .0
                            .iter()
                            .enumerate()
                            .map(|(i, f)| {
                                view! {
                                    <li>
                                        <Friend
                                            availability=signal(f.availability.clone()).0
                                            name=signal(f.name.to_string()).0
                                            status=signal(f.status.to_string()).0
                                            open_chat=open_new_chat
                                            order=Some(i)
                                        />
                                    </li>
                                }
                            })
                            .collect::<Vec<_>>()
                    }}
                </ul>
                <span class="mt-1 bold">"🔽 Offline"</span>
                <ul id="offline-list">
                    {move || {
                        friends
                            .get()
                            .1
                            .iter()
                            .map(|f| {
                                view! {
                                    <li>
                                        <Friend
                                            availability=signal(f.availability.clone()).0
                                            name=signal(f.name.to_string()).0
                                            status=signal(f.status.to_string()).0
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
        </div>
    }
}
