use crate::components::models::{Availability, ChatMessage, Friend, MessageStore, User};

use super::components::mainpage_component::MainPage;
use super::components::{
    chat_component::Chat, loginpage_component::LoginPage, register_component::RegisterPage,
};
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::components::{Route, Router, Routes};
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

    // Single global listener for all incoming XMPP messages.
    // Fixes: wrong payload path (event.payload.* not event.*), full-JID bare-JID mismatch,
    // and messages appearing in all open chat tabs instead of just the right one.
    {
        let messages = messages;
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

                    // Skip echo of own messages — already added optimistically on send
                    let own_jid = user.get_untracked().email;
                    if bare_jid == own_jid {
                        return;
                    }

                    let timestamp = payload["timestamp"].as_str().unwrap_or("").to_string();

                    let msg = ChatMessage {
                        from_jid: bare_jid.clone(),
                        body,
                        timestamp,
                        is_self: false,
                        pending: false,
                    };

                    messages.update(|store| {
                        let conv = store.entry(bare_jid).or_default();
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

    // sign_out: resets all session state so the next login starts clean.
    // Provided as a context so any component (MainPage) can call it.
    let sign_out = Callback::new(move |_: ()| {
        set_user.set(User {
            name: String::new(),
            email: String::new(),
            flavour_text: String::new(),
            availability: Availability::Offline,
        });
        set_friends.set((Vec::new(), Vec::new()));
        set_open_chats.set(Vec::new());
        messages.set(HashMap::new());
    });

    provide_context(user);
    provide_context(set_user);
    provide_context(friends);
    provide_context(set_friends);
    provide_context(open_chats);
    provide_context(set_open_chats);
    provide_context(messages);
    provide_context(sign_out);

    view! {
        <Router>
            <Routes fallback=move || view! { "404 Not Found" }>
                <Route path=path!("/") view=LoginPage />
                <Route path=path!("/register") view=RegisterPage />
                <Route path=path!("/main") view=MainPage />
                <Route path=path!("/chat/:id") view=Chat />
            </Routes>
        </Router>
    }
}
