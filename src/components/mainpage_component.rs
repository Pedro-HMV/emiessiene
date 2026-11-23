use crate::app::invoke;
use gloo_timers::callback::Interval;
use leptos::ev::{FocusEvent, KeyboardEvent};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::web_sys::HtmlInputElement;
use leptos_router::components::A;
use leptos_router::hooks::use_navigate;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = "invoke", catch)]
    async fn invoke_catching(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

use super::friend_component::Friend;
use super::models;
use models::{
    Availability, Friend, GroupInfo, GroupInvite, GroupStore, UpdateAvailabilityArgs,
    UpdateFlavourTextArgs, UpdateUsernameArgs, User,
};

#[derive(Serialize, Deserialize)]
struct AddContactArgs {
    jid: String,
}

#[derive(Serialize)]
struct SubscriptionArgs {
    jid: String,
}

#[derive(Serialize)]
struct FetchContactVcardArgs {
    jid: String,
}

#[derive(Serialize)]
struct CreateGroupArgs {
    #[serde(rename = "roomName")]
    room_name: String,
    invitees: Vec<String>,
}

#[derive(Serialize)]
struct JoinGroupArgs {
    #[serde(rename = "roomJid")]
    room_jid: String,
}

#[component]
pub fn MainPage() -> impl IntoView {
    let navigate = use_navigate();
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

    let presence_refresh = Interval::new(5000, {
        let set_friends = set_friends;
        move || {
            let set_friends = set_friends;
            spawn_local(async move {
                let cached = invoke(
                    "xmpp_get_presence_cache",
                    to_value(&serde_json::json!({})).unwrap(),
                )
                .await;
                if let Ok(presences) = from_value::<Vec<serde_json::Value>>(cached) {
                    set_friends.update(|(online, offline)| {
                        for presence in presences {
                            let jid = presence["jid"].as_str().unwrap_or("").to_string();
                            let availability = match presence["availability"].as_str() {
                                Some("Away") => Availability::Away,
                                Some("Busy") => Availability::Busy,
                                Some("Offline") => Availability::Offline,
                                _ => Availability::Online,
                            };
                            if jid.is_empty() {
                                continue;
                            }
                            let friend = online
                                .iter_mut()
                                .find(|friend| friend.email == jid)
                                .or_else(|| offline.iter_mut().find(|friend| friend.email == jid));
                            if let Some(friend) = friend {
                                friend.availability = availability;
                            }
                        }
                        let all: Vec<Friend> = online.drain(..).chain(offline.drain(..)).collect();
                        for friend in all {
                            if matches!(friend.availability, Availability::Offline) {
                                offline.push(friend);
                            } else {
                                online.push(friend);
                            }
                        }
                        online.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                        offline.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                    });
                }
            });
        }
    });
    std::mem::forget(presence_refresh);

    let (add_friend_jid, set_add_friend_jid) = signal(String::new());
    let (add_friend_status, set_add_friend_status) = signal(Option::<String>::None);
    let (group_route, set_group_route) = signal(Option::<String>::None);
    let (pending_requests, set_pending_requests) = signal(Vec::<String>::new());
    let (show_group_modal, set_show_group_modal) = signal(false);
    let (group_name, set_group_name) = signal(String::new());
    let (selected_group_members, set_selected_group_members) = signal(Vec::<String>::new());
    let (group_create_status, set_group_create_status) = signal(Option::<String>::None);
    let groups = use_context::<RwSignal<GroupStore>>().expect("No groups context");
    let pending_group_invites =
        use_context::<ReadSignal<Vec<GroupInvite>>>().expect("No pending group invites context");
    let set_pending_group_invites = use_context::<WriteSignal<Vec<GroupInvite>>>()
        .expect("No pending group invite setter context");
    Effect::new({
        let navigate = navigate.clone();
        move |_| {
            if let Some(route) = group_route.get() {
                navigate(&route, Default::default());
                set_group_route.set(None);
            }
        }
    });

    let add_friend = move |_| {
        let jid = add_friend_jid.get_untracked().trim().to_string();
        if jid.is_empty() {
            return;
        }
        set_add_friend_status.set(Some("Sending request...".to_string()));
        spawn_local(async move {
            match invoke_catching(
                "xmpp_add_contact",
                to_value(&AddContactArgs { jid: jid.clone() }).unwrap(),
            )
            .await
            {
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

    let toggle_group_member = move |jid: String| {
        set_selected_group_members.update(|selected| {
            if selected.contains(&jid) {
                selected.retain(|item| item != &jid);
            } else {
                selected.push(jid);
            }
        });
    };

    let create_group = move |_| {
        let name = group_name.get_untracked().trim().to_string();
        if name.is_empty() {
            set_group_create_status.set(Some("Group name is required".to_string()));
            return;
        }
        let invitees = selected_group_members.get_untracked();
        set_group_create_status.set(Some("Creating group...".to_string()));
        spawn_local(async move {
            match invoke_catching(
                "xmpp_create_group",
                to_value(&CreateGroupArgs {
                    room_name: name.clone(),
                    invitees,
                })
                .unwrap(),
            )
            .await
            {
                Ok(result) => match from_value::<serde_json::Value>(result) {
                    Ok(response) if response["success"].as_bool().unwrap_or(false) => {
                        let room_jid = response["room_jid"].as_str().unwrap_or("").to_string();
                        if !room_jid.is_empty() {
                            groups.update(|store| {
                                store.insert(
                                    room_jid.clone(),
                                    GroupInfo {
                                        room_jid: room_jid.clone(),
                                        name: name.clone(),
                                        owner: user.get_untracked().email,
                                        subject: Some(name.clone()),
                                        members: Vec::new(),
                                        muted: false,
                                        unread: 0,
                                    },
                                );
                            });
                            set_open_chats.update(|chats| {
                                if !chats.contains(&room_jid) {
                                    chats.push(room_jid.clone());
                                }
                            });
                            set_show_group_modal.set(false);
                            set_group_name.set(String::new());
                            set_selected_group_members.set(Vec::new());
                            set_group_create_status.set(Some(format!("Group '{}' created", name)));
                            set_group_route.set(Some(format!("/chat/{}", room_jid)));
                        }
                    }
                    Ok(response) => {
                        set_group_create_status.set(Some(
                            response["error"]
                                .as_str()
                                .unwrap_or("Failed to create group")
                                .to_string(),
                        ));
                    }
                    Err(error) => {
                        set_group_create_status
                            .set(Some(format!("Failed to create group: {error:?}")));
                    }
                },
                Err(error) => {
                    set_group_create_status.set(Some(format!("Failed to create group: {error:?}")));
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

    // Set up XMPP event listeners
    {
        let set_connection_status = set_connection_status;
        let set_friends = set_friends;
        let set_pending_requests = set_pending_requests;
        let user = user;
        let set_user = set_user;
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

            // Roster received: replace friends list with real XMPP roster entries,
            // then immediately replay presence cache so online contacts appear correctly.
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
                            online
                                .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                            offline
                                .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                            set_friends.set((online, offline));
                            // Immediately replay presence cache so contacts who are already
                            // online appear correctly. The roster response always resets
                            // everyone to Offline — this corrects it in the same microtask.
                            let set_friends = set_friends;
                            spawn_local(async move {
                                let cached = invoke(
                                    "xmpp_get_presence_cache",
                                    to_value(&serde_json::json!({})).unwrap(),
                                )
                                .await;
                                if let Ok(presences) = from_value::<Vec<serde_json::Value>>(cached)
                                {
                                    for p in presences {
                                        let jid = p["jid"].as_str().unwrap_or("").to_string();
                                        let avail_str =
                                            p["availability"].as_str().unwrap_or("Offline");
                                        let new_avail = match avail_str {
                                            "Online" => Availability::Online,
                                            "Away" => Availability::Away,
                                            "Busy" => Availability::Busy,
                                            _ => Availability::Offline,
                                        };
                                        if jid.is_empty() {
                                            continue;
                                        }
                                        set_friends.update(|(online, offline)| {
                                            let target =
                                                online.iter_mut().find(|f| f.email == jid).or_else(
                                                    || offline.iter_mut().find(|f| f.email == jid),
                                                );
                                            if let Some(f) = target {
                                                f.availability = new_avail.clone();
                                            }
                                            let all: Vec<Friend> =
                                                online.drain(..).chain(offline.drain(..)).collect();
                                            for f in all {
                                                if matches!(f.availability, Availability::Offline) {
                                                    offline.push(f);
                                                } else {
                                                    online.push(f);
                                                }
                                            }
                                            online.sort_by(|a, b| {
                                                a.name.to_lowercase().cmp(&b.name.to_lowercase())
                                            });
                                            offline.sort_by(|a, b| {
                                                a.name.to_lowercase().cmp(&b.name.to_lowercase())
                                            });
                                        });
                                        // Roster gives us the JID local-part as display name, not the
                                        // vCard nickname. For every already-online contact restored from
                                        // the cache, fetch their vCard now so the real nickname appears.
                                        if !matches!(new_avail, Availability::Offline) {
                                            let jid_clone = jid.clone();
                                            spawn_local(async move {
                                                let _ = invoke(
                                                    "xmpp_fetch_contact_vcard",
                                                    to_value(&FetchContactVcardArgs {
                                                        jid: jid_clone,
                                                    })
                                                    .unwrap(),
                                                )
                                                .await;
                                            });
                                        }
                                    }
                                }
                            });
                        }
                    }
                }
            })
                as Box<dyn Fn(JsValue)>);

            // Presence update: move friend between lists, update status, and fetch contact vCard
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
                            online
                                .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                            offline
                                .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                        });
                        // Fetch contact vCard on every presence event so name/flavour_text
                        // stays current (handles renames while we're online).
                        if !bare_jid.is_empty() {
                            let jid_for_vcard = bare_jid.clone();
                            spawn_local(async move {
                                let _ = invoke_catching(
                                    "xmpp_fetch_contact_vcard",
                                    to_value(&FetchContactVcardArgs { jid: jid_for_vcard })
                                        .unwrap(),
                                )
                                .await;
                            });
                        }
                    }
                }
            })
                as Box<dyn Fn(JsValue)>);

            // vCard received: update own user info OR a contact's name/flavour_text
            let vcard_callback = wasm_bindgen::closure::Closure::wrap(Box::new({
                let set_friends = set_friends;
                let user = user;
                let set_user = set_user;
                move |raw: JsValue| {
                    if let Ok(envelope) = from_value::<serde_json::Value>(raw) {
                        let payload = &envelope["payload"];
                        let vcard_jid = payload["jid"].as_str().unwrap_or("").to_string();
                        let nickname = payload["nickname"].as_str().unwrap_or("").to_string();
                        let flavour_text =
                            payload["flavour_text"].as_str().unwrap_or("").to_string();
                        let own_bare = user
                            .get_untracked()
                            .email
                            .split('/')
                            .next()
                            .unwrap_or("")
                            .to_string();
                        if vcard_jid.is_empty() || vcard_jid == own_bare {
                            // Own vCard — update user header
                            set_user.update(|u| {
                                if !nickname.is_empty() {
                                    u.name = nickname;
                                }
                                u.flavour_text = flavour_text;
                            });
                        } else {
                            // Contact vCard — update matching friend's name and flavour_text
                            set_friends.update(|(online, offline)| {
                                let target = online
                                    .iter_mut()
                                    .find(|f| f.email == vcard_jid)
                                    .or_else(|| offline.iter_mut().find(|f| f.email == vcard_jid));
                                if let Some(f) = target {
                                    if !nickname.is_empty() {
                                        f.name = nickname;
                                    }
                                    f.flavour_text = flavour_text;
                                }
                            });
                        }
                    }
                }
            })
                as Box<dyn Fn(JsValue)>);

            let _ = listen(
                "xmpp_vcard_received",
                vcard_callback.as_ref().unchecked_ref(),
            )
            .await;

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
                        // If this JID is already in our friends list, they are sending a
                        // counter-subscribe (we initiated the friendship first). Auto-accept
                        // so the subscription becomes mutual without requiring a second manual step.
                        let (online, offline) = friends.get_untracked();
                        let already_friend = online.iter().any(|f| f.email == from_jid)
                            || offline.iter().any(|f| f.email == from_jid);
                        if already_friend {
                            let jid = from_jid.clone();
                            spawn_local(async move {
                                let _ = invoke_catching(
                                    "xmpp_accept_subscription",
                                    to_value(&SubscriptionArgs { jid }).unwrap(),
                                )
                                .await;
                            });
                            return;
                        }
                        set_pending_requests.update(|reqs| {
                            if !reqs.contains(&from_jid) {
                                reqs.push(from_jid);
                            }
                        });
                    }
                }
            })
                as Box<dyn Fn(JsValue)>);
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
            })
                as Box<dyn Fn(JsValue)>);
            let _ = listen(
                "xmpp_roster_push",
                roster_push_callback.as_ref().unchecked_ref(),
            )
            .await;

            connected_callback.forget();
            disconnected_callback.forget();
            roster_callback.forget();
            presence_callback.forget();
            vcard_callback.forget();
            subscription_callback.forget();
            roster_push_callback.forget();

            // Now that all listeners are registered, request data that may have
            // been emitted before this page mounted and its listeners were ready.
            // These calls MUST come after all listen() calls above.
            // NOTE: xmpp_get_presence_cache is called inside roster_callback, not here,
            // because the roster response arrives async and resets all friends to Offline —
            // the cache must be replayed AFTER that reset, not before.
            let _ = invoke(
                "xmpp_request_roster",
                to_value(&serde_json::json!({})).unwrap(),
            )
            .await;
            let _ = invoke(
                "xmpp_fetch_vcard",
                to_value(&serde_json::json!({})).unwrap(),
            )
            .await;
            let _ = invoke(
                "xmpp_fetch_history",
                to_value(&serde_json::json!({})).unwrap(),
            )
            .await;
            let result = invoke(
                "get_pending_subscriptions",
                to_value(&serde_json::json!({})).unwrap(),
            )
            .await;
            if let Ok(jids) = from_value::<Vec<String>>(result) {
                for jid in jids {
                    set_pending_requests.update(|reqs| {
                        if !reqs.contains(&jid) {
                            reqs.push(jid);
                        }
                    });
                }
            }
            let result = invoke(
                "get_pending_group_invites",
                to_value(&serde_json::json!({})).unwrap(),
            )
            .await;
            if let Ok(invites) = from_value::<Vec<GroupInvite>>(result) {
                set_pending_group_invites.update(|pending| {
                    for invite in invites {
                        if !pending.iter().any(|item| item.room_jid == invite.room_jid) {
                            pending.push(invite);
                        }
                    }
                });
            }
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
                let display_name = if jid.contains("@conference.") {
                    groups
                        .get()
                        .get(&jid)
                        .map(|group| {
                            if group.unread == 0 {
                                group.name.clone()
                            } else {
                                format!("{} ({})", group.name, group.unread)
                            }
                        })
                        .unwrap_or_else(|| "Group".to_string())
                } else {
                    friend_name_for_jid(&jid)
                };
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
                            <img src="/assets/nto_icons_png/close.png" alt="Close chat" class="chat-tab-close-icon" />
                        </button>
                    </div>
                }
            })
            .collect::<Vec<_>>()
    };

    view! {
        <div id="main-container" class="flex-col">
            <div class="chat-tabs">
                <img src="/assets/nto-connected-brand-assets/nto-app-icon-64.png" alt="" class="chat-tabs-brand" />
                {chat_tabs}
            </div>
            <header>
                <div id="header_container" class="flex-row p-10 border-st">
                    <div id="header_left">
                        <div id="header_avatar">
                            <div id="avatar_img" style="width: 90px; height: 90px;">
                                <img src="/assets/nto_icons_png/profile.png" alt="Profile" class="header-profile-icon" />
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
                                    <button
                                        class="sign-out-link"
                                        style="background:none;border:none;padding:0;cursor:pointer;font:inherit;"
                                        on:click=move |_| {
                                            if let Some(sign_out) = use_context::<Callback<()>>() {
                                                sign_out.run(());
                                            }
                                            navigate("/", Default::default());
                                        }
                                    >
                                        "Sign Out"
                                    </button>
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
                            if jid.is_empty() {
                                return;
                            }
                            set_add_friend_status.set(Some("Sending request...".to_string()));
                            spawn_local(async move {
                                match invoke_catching(
                                        "xmpp_add_contact",
                                        to_value(&AddContactArgs { jid: jid.clone() }).unwrap(),
                                    )
                                    .await
                                {
                                    Ok(_) => {
                                        set_add_friend_status
                                            .set(Some(format!("Friend request sent to {}", jid)));
                                        set_add_friend_jid.set(String::new());
                                    }
                                    Err(e) => {
                                        set_add_friend_status.set(Some(format!("Error: {:?}", e)))
                                    }
                                }
                            });
                        }
                    }
                />
                <button on:click=add_friend title="Add friend">
                    "➕"
                </button>
                {move || {
                    add_friend_status
                        .get()
                        .map(|s| view! { <span class="add-friend-status">{s}</span> })
                }}
            </div>
            <div class="group-actions">
                <button
                    class="create-group-button"
                    on:click=move |_| set_show_group_modal.set(true)
                >
                    "＋ New group"
                </button>
            </div>
            <Show when=move || !pending_group_invites.get().is_empty()>
                <div class="pending-group-list">
                    <span class="bold">"Group invitations"</span>
                    <For
                        each=move || pending_group_invites.get()
                        key=|invite| invite.room_jid.clone()
                        children=move |invite| {
                            let accept_room = invite.room_jid.clone();
                            let accept_name = invite.room_name.clone();
                            let decline_room = invite.room_jid.clone();
                            view! {
                                <div class="pending-group-item">
                                    <span>{accept_name.clone()}</span>
                                    <button
                                        class="pending-accept"
                                        on:click=move |_| {
                                            let room_jid = accept_room.clone();
                                            let name = accept_name.clone();
                                            spawn_local(async move {
                                                let result = invoke_catching(
                                                    "xmpp_join_group",
                                                    to_value(&JoinGroupArgs {
                                                        room_jid: room_jid.clone(),
                                                    })
                                                    .unwrap(),
                                                )
                                                .await;
                                                if result.is_ok() {
                                                    groups.update(|store| {
                                                        store.insert(
                                                            room_jid.clone(),
                                                            GroupInfo {
                                                                room_jid: room_jid.clone(),
                                                                name,
                                                                owner: String::new(),
                                                                subject: None,
                                                                members: Vec::new(),
                                                                muted: false,
                                                                unread: 0,
                                                            },
                                                        );
                                                    });
                                                    set_pending_group_invites.update(|pending| {
                                                        pending.retain(|item| item.room_jid != room_jid);
                                                    });
                                                    set_open_chats.update(|chats| {
                                                        if !chats.contains(&room_jid) {
                                                            chats.push(room_jid.clone());
                                                        }
                                                    });
                                                    set_group_route.set(Some(format!("/chat/{}", room_jid)));
                                                }
                                            });
                                        }
                                    >
                                        "Accept"
                                    </button>
                                    <button
                                        class="pending-deny"
                                        on:click=move |_| {
                                            set_pending_group_invites.update(|pending| {
                                                pending.retain(|item| item.room_jid != decline_room);
                                            });
                                        }
                                    >
                                        "Decline"
                                    </button>
                                </div>
                            }
                        }
                    />
                </div>
            </Show>
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
                                let display_name = jid
                                    .split('@')
                                    .next()
                                    .unwrap_or(&jid)
                                    .to_string();
                                view! {
                                    <li class="pending-item">
                                        <span class="pending-avatar">
                                            <img src="/assets/nto_icons_png/profile.png" alt="" class="pending-profile-icon" />
                                        </span>
                                        <span class="pending-jid">{display_name}</span>
                                        <button
                                            class="pending-accept"
                                            title="Accept"
                                            on:click=move |_| {
                                                let jid = jid_accept.clone();
                                                let set_pending_requests = set_pending_requests;
                                                let set_friends = set_friends;
                                                spawn_local(async move {
                                                    let _ = invoke_catching(
                                                            "xmpp_accept_subscription",
                                                            to_value(
                                                                    &SubscriptionArgs {
                                                                        jid: jid.clone(),
                                                                    },
                                                                )
                                                                .unwrap(),
                                                        )
                                                        .await;
                                                    set_pending_requests
                                                        .update(|reqs| reqs.retain(|j| j != &jid));
                                                    set_friends
                                                        .update(|(online, offline)| {
                                                            let exists = online.iter().any(|f| f.email == jid)
                                                                || offline.iter().any(|f| f.email == jid);
                                                            if !exists {
                                                                let name = jid
                                                                    .split('@')
                                                                    .next()
                                                                    .unwrap_or(&jid)
                                                                    .to_string();
                                                                offline
                                                                    .push(Friend {
                                                                        name,
                                                                        email: jid.clone(),
                                                                        flavour_text: String::new(),
                                                                        availability: Availability::Offline,
                                                                    });
                                                                offline
                                                                    .sort_by(|a, b| {
                                                                        a.name.to_lowercase().cmp(&b.name.to_lowercase())
                                                                    });
                                                            }
                                                        });
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
                                                            to_value(
                                                                    &SubscriptionArgs {
                                                                        jid: jid.clone(),
                                                                    },
                                                                )
                                                                .unwrap(),
                                                        )
                                                        .await;
                                                    set_pending_requests
                                                        .update(|reqs| reqs.retain(|j| j != &jid));
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
            </div>
            <Show when=move || show_group_modal.get()>
                <div class="group-modal-backdrop">
                    <div class="group-modal" role="dialog" aria-modal="true" aria-labelledby="group-modal-title">
                        <div class="group-modal-header">
                            <h2 id="group-modal-title">"Create a group"</h2>
                            <button
                                type="button"
                                class="group-modal-close"
                                title="Close"
                                on:click=move |_| set_show_group_modal.set(false)
                            >
                                "×"
                            </button>
                        </div>
                        <label for="group-name">"Group name"</label>
                        <input
                            id="group-name"
                            type="text"
                            maxlength="80"
                            placeholder="Family, team, friends..."
                            prop:value=move || group_name.get()
                            on:input=move |ev| set_group_name.set(event_target_value(&ev))
                        />
                        <div class="group-member-heading">
                            "Invite online friends"
                            <span>{move || format!(" ({})", selected_group_members.get().len())}</span>
                        </div>
                        <div class="group-member-list">
                            {move || {
                                let (online, _) = friends.get();
                                online
                                    .into_iter()
                                    .map(|friend| {
                                        let jid = friend.email.clone();
                                        let jid_toggle = jid.clone();
                                        let selected = selected_group_members.get().contains(&jid);
                                        view! {
                                            <button
                                                type="button"
                                                class="group-member-option"
                                                class:selected=selected
                                                on:click=move |_| toggle_group_member(jid_toggle.clone())
                                            >
                                                <span class="group-member-avatar">"👤"</span>
                                                <span class="group-member-name">{friend.name.clone()}</span>
                                                <span class="group-member-check">
                                                    {if selected { "✓" } else { "" }}
                                                </span>
                                            </button>
                                        }
                                    })
                                    .collect::<Vec<_>>()
                            }}
                        </div>
                        {move || group_create_status.get().map(|status| view! {
                            <div class="group-create-status">{status}</div>
                        })}
                        <div class="group-modal-actions">
                            <button
                                type="button"
                                class="group-modal-cancel"
                                on:click=move |_| set_show_group_modal.set(false)
                            >
                                "Cancel"
                            </button>
                            <button
                                type="button"
                                class="group-modal-confirm"
                                on:click=create_group
                            >
                                "Create group"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}
