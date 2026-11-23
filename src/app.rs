use crate::components::models::{
    Availability, ChatMessage, Friend, GroupInvite, GroupMember, GroupStore, MessageStore, User,
};

use super::components::mainpage_component::MainPage;
use super::components::{
    chat_component::Chat, loginpage_component::LoginPage, register_component::RegisterPage,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos::web_sys;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::hooks::use_navigate;
use leptos_router::path;
use serde_wasm_bindgen::from_value;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    pub async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    async fn listen(event: &str, callback: &js_sys::Function) -> JsValue;
}

#[component]
pub fn App() -> impl IntoView {
    let (nudge_route, set_nudge_route) = signal(Option::<String>::None);
    let (user, set_user) = signal(User {
        name: "Username".to_string(),
        email: "user@localhost".to_string(),
        flavour_text: "Status message".to_string(),
        availability: Availability::Online,
    });
    let (friends, set_friends) = signal((Vec::<Friend>::new(), Vec::<Friend>::new()));
    // open_chats stores bare JIDs of currently open conversations
    let (open_chats, set_open_chats) = signal(Vec::<String>::new());
    // Global message store: bare JID → conversation history (capped at 200 messages each)
    let messages: RwSignal<MessageStore> = RwSignal::new(HashMap::new());
    let groups: RwSignal<GroupStore> = RwSignal::new(HashMap::new());
    let (pending_group_invites, set_pending_group_invites) = signal(Vec::<GroupInvite>::new());

    // Global MUC invite listener. It is intentionally separate from message handling
    // so invitations remain available even when no chat route is mounted.
    {
        let set_pending_group_invites = set_pending_group_invites;
        spawn_local(async move {
            let callback = Closure::wrap(Box::new(move |raw: JsValue| {
                if let Ok(envelope) = from_value::<serde_json::Value>(raw) {
                    if let Ok(invite) =
                        serde_json::from_value::<GroupInvite>(envelope["payload"].clone())
                    {
                        set_pending_group_invites.update(|invites| {
                            if !invites.iter().any(|item| item.room_jid == invite.room_jid) {
                                invites.push(invite);
                            }
                        });
                    }
                }
            }) as Box<dyn Fn(JsValue)>);

            let _ = listen(
                "xmpp_group_invite_received",
                callback.as_ref().unchecked_ref(),
            )
            .await;
            callback.forget();
        });
    }

    {
        let groups = groups;
        spawn_local(async move {
            let callback = Closure::wrap(Box::new(move |raw: JsValue| {
                if let Ok(envelope) = from_value::<serde_json::Value>(raw) {
                    let payload = &envelope["payload"];
                    let room_jid = payload["room_jid"].as_str().unwrap_or("").to_string();
                    let nickname = payload["nickname"].as_str().unwrap_or("").to_string();
                    let member_jid = payload["jid"].as_str().unwrap_or("").to_string();
                    let available = payload["available"].as_bool().unwrap_or(false);
                    let affiliation = payload["affiliation"].as_str().unwrap_or("member");
                    let role = payload["role"].as_str().unwrap_or("participant");
                    if room_jid.is_empty() || nickname.is_empty() {
                        return;
                    }
                    let availability = match payload["availability"].as_str() {
                        Some("Away") => Availability::Away,
                        Some("Busy") => Availability::Busy,
                        Some("Offline") => Availability::Offline,
                        _ => Availability::Online,
                    };
                    groups.update(|store| {
                        if let Some(group) = store.get_mut(&room_jid) {
                            if affiliation == "owner" && !member_jid.is_empty() {
                                group.owner = member_jid.clone();
                            }
                            if available {
                                if let Some(member) = group
                                    .members
                                    .iter_mut()
                                    .find(|item| item.nickname == nickname)
                                {
                                    member.availability = availability;
                                    if !member_jid.is_empty() {
                                        member.jid = member_jid.clone();
                                    }
                                    member.role =
                                        if affiliation == "owner" || affiliation == "admin" {
                                            affiliation.to_string()
                                        } else {
                                            role.to_string()
                                        };
                                } else {
                                    group.members.push(GroupMember {
                                        jid: member_jid,
                                        nickname,
                                        role: if affiliation == "owner" || affiliation == "admin" {
                                            affiliation.to_string()
                                        } else {
                                            role.to_string()
                                        },
                                        availability,
                                    });
                                }
                            } else {
                                group.members.retain(|item| item.nickname != nickname);
                            }
                        }
                    });
                }
            }) as Box<dyn Fn(JsValue)>);

            let _ = listen(
                "xmpp_group_member_update",
                callback.as_ref().unchecked_ref(),
            )
            .await;
            callback.forget();
        });
    }

    // Single global listener for all incoming XMPP messages.
    // Fixes: wrong payload path (event.payload.* not event.*), full-JID bare-JID mismatch,
    // and messages appearing in all open chat tabs instead of just the right one.
    {
        let messages = messages;
        let groups = groups;
        let open_chats = open_chats;
        spawn_local(async move {
            let callback = Closure::wrap(Box::new(move |raw: JsValue| {
                // Tauri wraps events as: { event: "...", payload: { ... }, id: ... }
                if let Ok(envelope) = from_value::<serde_json::Value>(raw) {
                    let payload = &envelope["payload"];
                    let from_full = payload["from"].as_str().unwrap_or("").to_string();
                    let body = match payload["body"].as_str() {
                        Some(b) if !b.is_empty() => b.to_string(),
                        _ => return,
                    };

                    // Strip resource to get bare JID
                    let bare_jid = from_full
                        .split('/')
                        .next()
                        .unwrap_or(&from_full)
                        .to_string();

                    let timestamp = payload["timestamp"].as_str().unwrap_or("").to_string();
                    let message_id = payload["id"].as_str().unwrap_or("").to_string();
                    let is_self = payload["is_self"].as_bool().unwrap_or(false);
                    let author_nickname = payload["author_nickname"].as_str().map(str::to_string);
                    let group_room = payload["group_room"].as_str();
                    let peer_jid = if let Some(room_jid) = group_room {
                        room_jid.split('/').next().unwrap_or(room_jid).to_string()
                    } else if is_self {
                        payload["to"]
                            .as_str()
                            .unwrap_or("")
                            .split('/')
                            .next()
                            .unwrap_or("")
                            .to_string()
                    } else {
                        bare_jid.clone()
                    };

                    let msg = ChatMessage {
                        id: message_id.clone(),
                        from_jid: bare_jid.clone(),
                        body,
                        timestamp,
                        is_self,
                        pending: false,
                        author_nickname,
                    };

                    if group_room.is_some() && !is_self {
                        let room_jid = peer_jid.clone();
                        let active = open_chats.get_untracked().contains(&room_jid);
                        if !active {
                            groups.update(|store| {
                                if let Some(group) = store.get_mut(&room_jid) {
                                    group.unread = group.unread.saturating_add(1);
                                }
                            });
                        }
                    }

                    messages.update(|store| {
                        if peer_jid.is_empty() {
                            return;
                        }
                        let conv = store.entry(peer_jid).or_default();
                        if !message_id.is_empty()
                            && conv.iter().any(|existing| existing.id == message_id)
                        {
                            return;
                        }
                        if let Some(existing) = conv.iter_mut().find(|existing| {
                            existing.pending
                                && existing.body == msg.body
                                && existing.is_self == msg.is_self
                        }) {
                            existing.pending = false;
                            if existing.id.is_empty() {
                                existing.id = message_id;
                            }
                            if existing.timestamp.is_empty() {
                                existing.timestamp = msg.timestamp;
                            }
                            return;
                        }
                        if conv.iter().any(|existing| {
                            existing.id.is_empty()
                                && !existing.pending
                                && existing.body == msg.body
                                && existing.is_self == msg.is_self
                        }) {
                            return;
                        }
                        conv.push(msg);
                        // Cap each conversation at 200 messages to avoid memory growth
                        if conv.len() > 200 {
                            conv.drain(0..conv.len() - 200);
                        }
                    });
                }
            }) as Box<dyn Fn(JsValue)>);

            let _ = listen("xmpp_message_received", callback.as_ref().unchecked_ref()).await;
            callback.forget();
        });
    }

    {
        let set_pending_group_invites = set_pending_group_invites;
        spawn_local(async move {
            let callback = Closure::wrap(Box::new(move |raw: JsValue| {
                if let Ok(envelope) = from_value::<serde_json::Value>(raw) {
                    if let Ok(invite) =
                        serde_json::from_value::<GroupInvite>(envelope["payload"].clone())
                    {
                        set_pending_group_invites.update(|invites| {
                            if !invites.iter().any(|item| item.room_jid == invite.room_jid) {
                                invites.push(invite);
                            }
                        });
                    }
                }
            }) as Box<dyn Fn(JsValue)>);

            let _ = listen(
                "xmpp_group_invite_received",
                callback.as_ref().unchecked_ref(),
            )
            .await;
            callback.forget();
        });
    }

    {
        let set_open_chats = set_open_chats;
        let set_nudge_route = set_nudge_route;
        spawn_local(async move {
            let callback = Closure::wrap(Box::new(move |raw: JsValue| {
                if let Ok(envelope) = from_value::<serde_json::Value>(raw) {
                    let from_full = envelope["payload"]["from"]
                        .as_str()
                        .unwrap_or("")
                        .to_string();
                    let from_bare = from_full
                        .split('/')
                        .next()
                        .unwrap_or(&from_full)
                        .to_string();
                    if from_bare.is_empty() {
                        return;
                    }
                    set_open_chats.update(|chats| {
                        if !chats.contains(&from_bare) {
                            chats.push(from_bare.clone());
                        }
                    });
                    set_nudge_route.set(Some(format!("/chat/{}", from_bare)));
                    if let Some(window) = web_sys::window() {
                        if let Some(document) = window.document() {
                            spawn_local(async move {
                                gloo_timers::future::TimeoutFuture::new(60).await;
                                if let Some(chat_container) =
                                    document.query_selector(".chat_container").ok().flatten()
                                {
                                    let _ = chat_container.class_list().add_1("nto-nudge-shake");
                                    let chat_container_for_cleanup = chat_container.clone();
                                    spawn_local(async move {
                                        gloo_timers::future::TimeoutFuture::new(650).await;
                                        let _ = chat_container_for_cleanup
                                            .class_list()
                                            .remove_1("nto-nudge-shake");
                                    });
                                }
                            });
                        }
                    }
                }
            }) as Box<dyn Fn(JsValue)>);

            let _ = listen("xmpp_nudge_received", callback.as_ref().unchecked_ref()).await;
            callback.forget();
        });
    }

    // sign_out: resets all session state so the next login starts clean.
    // Provided as a context so any component (MainPage) can call it.
    let sign_out = Callback::new(move |_: ()| {
        spawn_local(async move {
            let args = JsValue::null();
            let _ = invoke("xmpp_disconnect", args).await;
        });
        set_user.set(User {
            name: String::new(),
            email: String::new(),
            flavour_text: String::new(),
            availability: Availability::Offline,
        });
        set_friends.set((Vec::new(), Vec::new()));
        set_open_chats.set(Vec::new());
        messages.set(HashMap::new());
        groups.set(HashMap::new());
        set_pending_group_invites.set(Vec::new());
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.session_storage() {
                let _ = storage.set_item("nto_skip_auto_cli_login", "1");
            }
        }
    });

    provide_context(user);
    provide_context(set_user);
    provide_context(friends);
    provide_context(set_friends);
    provide_context(open_chats);
    provide_context(set_open_chats);
    provide_context(messages);
    provide_context(groups);
    provide_context(pending_group_invites);
    provide_context(set_pending_group_invites);
    provide_context(sign_out);

    let nudge_route_view = nudge_route;
    let set_nudge_route_view = set_nudge_route;

    view! {
        <Router>
            {move || {
                let navigate = use_navigate();
                let route = nudge_route_view.get();
                if let Some(route) = route {
                    navigate(&route, Default::default());
                    set_nudge_route_view.set(None);
                }
                view! { <></> }
            }}
            <Routes fallback=move || view! { "404 Not Found" }>
                <Route path=path!("/") view=LoginPage />
                <Route path=path!("/register") view=RegisterPage />
                <Route path=path!("/main") view=MainPage />
                <Route path=path!("/chat/:id") view=Chat />
            </Routes>
        </Router>
    }
}
