use crate::app::invoke;
use leptos::ev::{FocusEvent, KeyboardEvent};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::web_sys::HtmlInputElement;
use leptos_router::components::A;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "tauri"], js_name = "invoke", catch)]
    async fn invoke_catching(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

use super::friend_component::Friend;
use super::models;
use models::{
    Availability, Friend, UpdateAvailabilityArgs, UpdateFlavourTextArgs, UpdateUsernameArgs, User,
};

#[derive(Serialize, Deserialize)]
struct AddContactArgs {
    jid: String,
}

#[derive(Serialize)]
struct SubscriptionArgs {
    jid: String,
}

#[component]
pub fn MainPage() -> impl IntoView {
    let (editing_user, set_editing_user) = signal(false);
    let (editing_flavour_text, set_editing_flavour_text) = signal(false);
    let (editing_availability, set_editing_availability) = signal(false);
    let flavour_input_ref = NodeRef::<leptos::html::Input>::new();
    let (connection_status, set_connection_status) =
        signal("🔌 Checking connection...".to_string());

    // Use the global user context instead of creating a local one
    let user = use_context::<ReadSignal<User>>().expect("No user context");
    let set_user = use_context::<WriteSignal<User>>().expect("No user setter context");

    let friends =
        use_context::<ReadSignal<(Vec<Friend>, Vec<Friend>)>>().expect("No friends context");

    let open_chats =
        move || use_context::<ReadSignal<Vec<String>>>().expect("No open chats context");

    let set_open_chats =
        use_context::<WriteSignal<Vec<String>>>().expect("No set open chats context");

    let set_friends =
        use_context::<WriteSignal<(Vec<Friend>, Vec<Friend>)>>().expect("No set friends context");

    let (add_friend_jid, set_add_friend_jid) = signal(String::new());
    let (add_friend_status, set_add_friend_status) = signal(Option::<String>::None);
    let (pending_requests, set_pending_requests) = signal(Vec::<String>::new());

    let add_friend = move |_| {
        let jid = add_friend_jid.get_untracked().trim().to_string();
        if jid.is_empty() {
            return;
        }
        set_add_friend_status.set(Some("Sending request...".to_string()));
        spawn_local(async move {
            match invoke_catching("xmpp_add_contact", to_value(&AddContactArgs { jid: jid.clone() }).unwrap()).await {
                Ok(_) => {
                    set_add_friend_status.set(Some(format!("Friend request sent to {}", jid)));
                    set_add_friend_jid.set(String::new());
                }
                Err(e) => {
                    let msg = format!("Error: {:?}", e);
                    set_add_friend_status.set(Some(msg));
                }
            }
        });
    };

    // Check XMPP connection status on page load
    {
        let set_connection_status = set_connection_status;
        spawn_local(async move {
            match invoke(
                "xmpp_get_connection_status",
                to_value(&serde_json::json!({})).unwrap(),
            )
            .await
            {
                result => match from_value::<serde_json::Value>(result) {
                    Ok(status) => {
                        if status["connected"].as_bool().unwrap_or(false) {
                            set_connection_status.set("Connected".to_string());
                        } else {
                            set_connection_status.set("Disconnected".to_string());
                        }
                    }
                    Err(_) => {
                        set_connection_status.set("Disconnected".to_string());
                    }
                },
            }
        });
    }

    // Fetch any subscription requests that arrived before this page mounted
    {
        spawn_local(async move {
            let result = invoke(
                "get_pending_subscriptions",
                to_value(&serde_json::json!({})).unwrap(),
            )
            .await;
            if let Ok(jids) = from_value::<Vec<String>>(result) {
                if !jids.is_empty() {
                    set_pending_requests.update(|reqs| {
                        for jid in jids {
                            if !reqs.contains(&jid) {
                                reqs.push(jid);
                            }
                        }
                    });
                }
            }
        });
    }

    // Re-request the roster now that listeners are mounted.
    // The initial roster fetch happens right after login, but the page isn't
    // mounted yet at that point — this call ensures we always get fresh data.
    {
        spawn_local(async move {
            let _ = invoke(
                "xmpp_request_roster",
                to_value(&serde_json::json!({})).unwrap(),
            )
            .await;
        });
    }

    // Set up XMPP event listeners
    {
        let set_connection_status = set_connection_status;
        let set_friends = set_friends;
        let set_pending_requests = set_pending_requests;
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
                    set_connection_status.set("Connected".to_string());
                }
            })
                as Box<dyn Fn(JsValue)>);

            let disconnected_callback = wasm_bindgen::closure::Closure::wrap(Box::new({
                let set_connection_status = set_connection_status;
                move |_event: JsValue| {
                    set_connection_status.set("Disconnected".to_string());
                }
            })
                as Box<dyn Fn(JsValue)>);

            // Roster received: replace friends list with real XMPP roster entries
            let roster_callback = wasm_bindgen::closure::Closure::wrap(Box::new({
                let set_friends = set_friends;
                move |raw: JsValue| {
                    if let Ok(envelope) = from_value::<serde_json::Value>(raw) {
                        let payload = &envelope["payload"];
                        if let Some(items) = payload.as_array() {
                            let mut online: Vec<Friend> = Vec::new();
                            let mut offline: Vec<Friend> = Vec::new();
                            for item in items {
                                let jid = item["jid"].as_str().unwrap_or("").to_string();
                                let name = item["name"]
                                    .as_str()
                                    .filter(|s| !s.is_empty())
                                    .map(|s| s.to_string())
                                    .unwrap_or_else(|| {
                                        jid.split('@').next().unwrap_or(&jid).to_string()
                                    });
                                let sub = item["subscription"].as_str().unwrap_or("");
                                // Only include mutual contacts
                                if sub == "both" || sub == "from" || sub == "to" {
                                    offline.push(Friend {
                                        name,
                                        email: jid,
                                        flavour_text: String::new(),
                                        availability: Availability::Offline,
                                    });
                                }
                            }
                            // Sort both lists A→Z by display name
                            online.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                            offline.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                            set_friends.set((online, offline));
                        }
                    }
                }
            })
                as Box<dyn Fn(JsValue)>);

            // Presence update: move friend between lists and update status text
            let presence_callback = wasm_bindgen::closure::Closure::wrap(Box::new({
                let set_friends = set_friends;
                move |raw: JsValue| {
                    if let Ok(envelope) = from_value::<serde_json::Value>(raw) {
                        let payload = &envelope["payload"];
                        let jid_full = payload["jid"].as_str().unwrap_or("").to_string();
                        let bare_jid = jid_full.split('/').next().unwrap_or(&jid_full).to_string();
                        let avail_str = payload["availability"].as_str().unwrap_or("Offline");
                        let status_text = payload["status"].as_str().unwrap_or("").to_string();
                        let new_avail = match avail_str {
                            "Online" => Availability::Online,
                            "Away" => Availability::Away,
                            "Busy" => Availability::Busy,
                            _ => Availability::Offline,
                        };
                        set_friends.update(|(online, offline)| {
                            // Find the friend in either list and update in place
                            let found_online = online.iter_mut().find(|f| f.email == bare_jid);
                            let found_offline = offline.iter_mut().find(|f| f.email == bare_jid);
                            let friend = found_online.or(found_offline);
                            if let Some(f) = friend {
                                f.availability = new_avail.clone();
                                f.flavour_text = status_text;
                            }
                            // Re-sort online/offline split
                            let all: Vec<Friend> =
                                online.drain(..).chain(offline.drain(..)).collect();
                            for f in all {
                                if matches!(f.availability, Availability::Offline) {
                                    offline.push(f);
                                } else {
                                    online.push(f);
                                }
                            }
                            online.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                            offline.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                        });
                    }
                }
            })
                as Box<dyn Fn(JsValue)>);

            let _ = listen(
                "xmpp_connected",
                connected_callback.as_ref().unchecked_ref(),
            )
            .await;
            let _ = listen(
                "xmpp_disconnected",
                disconnected_callback.as_ref().unchecked_ref(),
            )
            .await;
            let _ = listen(
                "xmpp_roster_received",
                roster_callback.as_ref().unchecked_ref(),
            )
            .await;
            let _ = listen(
                "xmpp_presence_update",
                presence_callback.as_ref().unchecked_ref(),
            )
            .await;

            // Listen for incoming subscription requests (pending friend requests)
            let subscription_callback = wasm_bindgen::closure::Closure::wrap(Box::new({
                let set_pending_requests = set_pending_requests;
                let friends = friends;
                move |raw: JsValue| {
                    if let Ok(envelope) = from_value::<serde_json::Value>(raw) {
                        let from_jid = envelope["payload"]["from_jid"]
                            .as_str()
                            .unwrap_or("")
                            .to_string();
                        if from_jid.is_empty() {
                            return;
                        }
                        // Skip if we already have this contact in the friends list
                        let (online, offline) = friends.get_untracked();
                        let already_friend = online.iter().any(|f| f.email == from_jid)
                            || offline.iter().any(|f| f.email == from_jid);
                        if already_friend {
                            return;
                        }
                        set_pending_requests.update(|reqs| {
                            if !reqs.contains(&from_jid) {
                                reqs.push(from_jid);
                            }
                        });
                    }
                }
            }) as Box<dyn Fn(JsValue)>);
            let _ = listen(
                "xmpp_subscription_request",
                subscription_callback.as_ref().unchecked_ref(),
            )
            .await;

            // Listen for roster push (contact added / subscription state changed)
            let roster_push_callback = wasm_bindgen::closure::Closure::wrap(Box::new({
                let set_friends = set_friends;
                let set_pending_requests = set_pending_requests;
                move |raw: JsValue| {
                    if let Ok(envelope) = from_value::<serde_json::Value>(raw) {
                        let payload = &envelope["payload"];
                        let jid = payload["jid"].as_str().unwrap_or("").to_string();
                        let name_raw = payload["name"].as_str().unwrap_or("").to_string();
                        let sub = payload["subscription"].as_str().unwrap_or("").to_string();
                        if jid.is_empty() {
                            return;
                        }
                        let name = if name_raw.is_empty() {
                            jid.split('@').next().unwrap_or(&jid).to_string()
                        } else {
                            name_raw
                        };
                        if sub == "both" || sub == "from" || sub == "to" {
                            set_friends.update(|(online, offline)| {
                                let exists = online.iter().any(|f| f.email == jid)
                                    || offline.iter().any(|f| f.email == jid);
                                if !exists {
                                    offline.push(Friend {
                                        name,
                                        email: jid.clone(),
                                        flavour_text: String::new(),
                                        availability: Availability::Offline,
                                    });
                                    offline.sort_by(|a, b| {
                                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                                    });
                                }
                            });
                            // Remove from pending — this JID is now a confirmed contact
                            set_pending_requests.update(|reqs| reqs.retain(|j| j != &jid));
                        }
                    }
                }
            }) as Box<dyn Fn(JsValue)>);
            let _ = listen(
                "xmpp_roster_push",
                roster_push_callback.as_ref().unchecked_ref(),
            )
            .await;

            connected_callback.forget();
            disconnected_callback.forget();
            roster_callback.forget();
            presence_callback.forget();
            subscription_callback.forget();
            roster_push_callback.forget();
        });
    }

    let online_friends = move || friends.get().0;
    let offline_friends = move || friends.get().1;

    // Close a chat tab by JID
    let close_chat = move |jid: String| {
        set_open_chats.update(|chats| {
            chats.retain(|j| j != &jid);
        });
    };

    // Helper: look up a friend's display name by JID
    let friend_name_for_jid = move |jid: &str| -> String {
        let (online, offline) = friends.get();
        online
            .iter()
            .chain(offline.iter())
            .find(|f| f.email == jid)
            .map(|f| f.name.clone())
            .unwrap_or_else(|| jid.to_string())
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

    let blur_on_enter_ft = move |ev: KeyboardEvent| {
        if ev.key() == "Enter" {
            let target = ev.target().unwrap();
            let el: &HtmlInputElement = target.dyn_ref().expect("Failed to get input element");
            el.blur().unwrap();
        }
    };

    // Auto-focus the flavour text input whenever editing mode is activated.
    // spawn_local defers one microtask tick, giving <Show> time to mount the <input>
    // before we call focus() on it via the NodeRef.
    Effect::new(move |_| {
        if editing_flavour_text.get() {
            spawn_local(async move {
                if let Some(el) = flavour_input_ref.get_untracked() {
                    let _ = el.focus();
                }
            });
        }
    });

    let update_flavour_text_handler = move |ev: FocusEvent| {
        ev.prevent_default();
        let ft = event_target_value(&ev);
        leptos::logging::log!("update_flavour_text_handler fired, value = {:?}", ft);
        // Update signal immediately so the UI reflects it without waiting for the backend
        set_user.update(|u| u.flavour_text = ft.clone());
        set_editing_flavour_text.set(false);
        // Persist in the background
        spawn_local(async move {
            match invoke_catching(
                "update_flavour_text",
                to_value(&UpdateFlavourTextArgs { flavour_text: &ft }).unwrap(),
            )
            .await
            {
                Ok(_) => leptos::logging::log!("update_flavour_text saved ok"),
                Err(e) => leptos::logging::warn!("update_flavour_text failed: {:?}", e),
            }
        });
    };

    let select_availability = move |avail: Availability| {
        set_editing_availability.set(false);
        spawn_local(async move {
            let args = UpdateAvailabilityArgs {
                availability: avail,
            };
            let result = invoke("update_availability", to_value(&args).unwrap()).await;
            if let Ok(updated_user) = from_value::<User>(result) {
                set_user.update(|u| {
                    u.availability = updated_user.availability;
                });
            }
        });
    };

    let chat_tabs = move || {
        open_chats()
            .get()
            .iter()
            .map(|jid| {
                let jid = jid.clone();
                let display_name = friend_name_for_jid(&jid);
                let jid_close = jid.clone();
                let close_chat = close_chat.clone();
                view! {
                    <div class="chat-tab-container">
                        <A href=move || format!("/chat/{}", jid)>
                            <button class="chat-tab">{display_name}</button>
                        </A>
                        <button
                            class="chat-tab-close"
                            on:click=move |_| close_chat(jid_close.clone())
                        >
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
                                style="width: 90px; height: 90px;"
                            >
                            </div>
                        </div>
                    </div>
                    <div id="header_right" class="ml-1">
                        <div id="header_info">
                            <div class="user-name-row">
                                <Show
                                    when=move || { editing_user.get() }
                                    fallback=move || {
                                        view! {
                                            <span
                                                on:click=move |_| set_editing_user.set(true)
                                                class="user-display-name"
                                            >
                                                {move || user.get().name}
                                            </span>
                                        }
                                    }
                                >
                                    <input
                                        on:blur=update_username
                                        on:keydown=blur_on_enter
                                        class="user-edit_input user-name-input"
                                        value=move || user.get().name
                                    />
                                </Show>
                                <Show
                                    when=move || editing_availability.get()
                                    fallback=move || {
                                        view! {
                                            <span class="user-availability-parens">
                                                " ("
                                                <span class="user-availability-text">
                                                    {move || user.get().availability.to_string()}
                                                </span> ")"
                                            </span>
                                        }
                                    }
                                >
                                    <div class="avail-dropdown">
                                        <button
                                            class="avail-option"
                                            on:click=move |_| select_availability(Availability::Online)
                                        >
                                            "Online"
                                        </button>
                                        <button
                                            class="avail-option"
                                            on:click=move |_| select_availability(Availability::Away)
                                        >
                                            "Away"
                                        </button>
                                        <button
                                            class="avail-option"
                                            on:click=move |_| select_availability(Availability::Busy)
                                        >
                                            "Busy"
                                        </button>
                                        <button
                                            class="avail-option"
                                            on:click=move |_| select_availability(Availability::Offline)
                                        >
                                            "Offline"
                                        </button>
                                    </div>
                                </Show>
                                <button
                                    class="avail-arrow-btn"
                                    on:click=move |_| set_editing_availability.update(|v| *v = !*v)
                                >
                                    "▾"
                                </button>
                            </div>
                            <div class="user-flavour-row">
                                <Show
                                    when=move || editing_flavour_text.get()
                                    fallback=move || {
                                        view! {
                                            <span
                                                class="user-flavour-text"
                                                class:user-flavour-placeholder=move || {
                                                    user.get().flavour_text.is_empty()
                                                }
                                                on:click=move |_| set_editing_flavour_text.set(true)
                                            >
                                                {move || {
                                                    let ft = user.get().flavour_text;
                                                    if ft.is_empty() {
                                                        "<Type a personal message>".to_string()
                                                    } else {
                                                        ft
                                                    }
                                                }}
                                            </span>
                                        }
                                    }
                                >
                                    <input
                                        node_ref=flavour_input_ref
                                        on:blur=update_flavour_text_handler
                                        on:keydown=blur_on_enter_ft
                                        class="user-edit_input user-flavour-input"
                                        value=move || user.get().flavour_text
                                    />
                                </Show>
                            </div>
                            <div class="user-status-row">
                                <span class="xmpp-dot">
                                    {move || {
                                        if connection_status.get().contains("Connected") {
                                            "🟢"
                                        } else {
                                            "🔴"
                                        }
                                    }}
                                </span>
                                <span class="xmpp-label">{move || connection_status.get()}</span>
                                <span class="status-sep">"|"</span>
                                <span class="sign-out-link">
                                    <A href="/">"Sign Out"</A>
                                </span>
                            </div>
                        </div>
                    </div>
                </div>
            </header>
            <div id="find-friends" class="mt-1 mb-1 bg-white border-1b">
                <input
                    type="text"
                    id="find-friend_input"
                    placeholder="Find a friend"
                    class="border-1b"
                    prop:value=move || add_friend_jid.get()
                    on:input=move |ev| set_add_friend_jid.set(event_target_value(&ev))
                    on:keydown=move |ev: KeyboardEvent| {
                        if ev.key() == "Enter" {
                            ev.prevent_default();
                            let jid = add_friend_jid.get_untracked().trim().to_string();
                            if jid.is_empty() { return; }
                            set_add_friend_status.set(Some("Sending request...".to_string()));
                            spawn_local(async move {
                                match invoke_catching("xmpp_add_contact", to_value(&AddContactArgs { jid: jid.clone() }).unwrap()).await {
                                    Ok(_) => {
                                        set_add_friend_status.set(Some(format!("Friend request sent to {}", jid)));
                                        set_add_friend_jid.set(String::new());
                                    }
                                    Err(e) => set_add_friend_status.set(Some(format!("Error: {:?}", e))),
                                }
                            });
                        }
                    }
                />
                <button on:click=add_friend title="Add friend">"➕"</button>
                {move || add_friend_status.get().map(|s| view! { <span class="add-friend-status">{s}</span> })}
            </div>
            <div id="friends-container" class="flex-col flex-grow bg-white auto-y">
                // Pending friend requests section (styled like Friends/Offline groups)
                <Show when=move || !pending_requests.get().is_empty()>
                    <span class="bold">
                        "📩 Pending"
                        <span class="group-count">
                            {move || format!(" ({})", pending_requests.get().len())}
                        </span>
                    </span>
                    <ul class="contact-list" id="pending-list">
                        <For
                            each=move || pending_requests.get()
                            key=|jid| jid.clone()
                            children=move |jid| {
                                let jid_accept = jid.clone();
                                let jid_deny = jid.clone();
                                let set_pending_requests = set_pending_requests;
                                let display_name = jid.split('@').next().unwrap_or(&jid).to_string();
                                view! {
                                    <li class="pending-item">
                                        <span class="pending-avatar">"👤"</span>
                                        <span class="pending-jid">{display_name}</span>
                                        <button
                                            class="pending-accept"
                                            title="Accept"
                                            on:click=move |_| {
                                                let jid = jid_accept.clone();
                                                let set_pending_requests = set_pending_requests;
                                                spawn_local(async move {
                                                    let _ = invoke_catching(
                                                        "xmpp_accept_subscription",
                                                        to_value(&SubscriptionArgs { jid: jid.clone() }).unwrap(),
                                                    )
                                                    .await;
                                                    set_pending_requests.update(|reqs| reqs.retain(|j| j != &jid));
                                                });
                                            }
                                        >
                                            "✔"
                                        </button>
                                        <button
                                            class="pending-deny"
                                            title="Decline"
                                            on:click=move |_| {
                                                let jid = jid_deny.clone();
                                                let set_pending_requests = set_pending_requests;
                                                spawn_local(async move {
                                                    let _ = invoke_catching(
                                                        "xmpp_deny_subscription",
                                                        to_value(&SubscriptionArgs { jid: jid.clone() }).unwrap(),
                                                    )
                                                    .await;
                                                    set_pending_requests.update(|reqs| reqs.retain(|j| j != &jid));
                                                });
                                            }
                                        >
                                            "✘"
                                        </button>
                                    </li>
                                }
                            }
                        />
                    </ul>
                </Show>
                <span class="group-header bold">"Friends"</span>
                <ul id="online-list">
                    <For
                        each=move || online_friends()
                        key=|f| f.email.clone()
                        children=move |friend| {
                            let jid = friend.email.clone();
                            view! {
                                <li>
                                    <A href=move || format!("/chat/{}", jid)>
                                        <Friend
                                            availability=signal(friend.availability).0
                                            name=signal(friend.name).0
                                            flavour_text=signal(friend.flavour_text).0
                                        />
                                    </A>
                                </li>
                            }
                        }
                    />
                </ul>
                <span class="group-header bold">"Offline"</span>
                <ul id="offline-list">
                    <For
                        each=move || offline_friends()
                        key=|f| f.email.clone()
                        children=move |friend| {
                            view! {
                                <li>
                                    <Friend
                                        availability=signal(friend.availability).0
                                        name=signal(friend.name).0
                                        flavour_text=signal(friend.flavour_text).0
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
