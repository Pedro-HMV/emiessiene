use crate::app::invoke;
use leptos::ev::{FocusEvent, KeyboardEvent};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::web_sys::HtmlInputElement;
use leptos_router::components::A;
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::{JsCast, JsValue};

use super::friend_component::Friend;
use super::models;
use models::{Availability, Friend, UpdateUsernameArgs, User};

#[component]
pub fn MainPage() -> impl IntoView {
    let (editing_user, set_editing_user) = signal(false);
    let (user, set_user) = signal(User {
        name: "Username".to_string(),
        email: "user@hotmail.com".to_string(),
        status: "Status message".to_string(),
        availability: Availability::Online,
    });

    let friends =
        use_context::<ReadSignal<(Vec<Friend>, Vec<Friend>)>>().expect("No friends context");

    let open_chats =
        move || use_context::<ReadSignal<Vec<usize>>>().expect("No open chats context");

    let online_friends = move || friends.get().0;
    let offline_friends = move || friends.get().1;

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
                view! {
                    <A href=move || {format!("/chat/{}", id)} >
                    <button class="chat-tab">
                        {friend.name}
                    </button>
                    </A>
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
                    <For each=move || {online_friends().clone().into_iter().enumerate().collect::<Vec<_>>() }
                        key=|f| f.0
                        children=move |(id, friend)| {
                            let friend = friend.clone();
                            view! {
                                <li>
                                <A href=move|| format!("/chat/{id}", id=id) >
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
                <For each=move || {offline_friends().clone().into_iter().enumerate().collect::<Vec<_>>() }
                key=|f| f.0
                children=move |(_,friend)| {
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
