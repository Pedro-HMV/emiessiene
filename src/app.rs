use crate::components::models::{Availability, Friend, User};

use super::components::mainpage_component::MainPage;
use super::components::{chat_component::Chat, loginpage_component::LoginPage};
use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;
use serde_wasm_bindgen::from_value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "tauri"])]
    pub async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[component]
pub fn App() -> impl IntoView {
    let (user, set_user) = signal(User {
        name: "Username".to_string(),
        email: "user@hotmail.com".to_string(),
        status: "Status message".to_string(),
        availability: Availability::Online,
    });
    let (friends, set_friends) = signal((Vec::<Friend>::new(), Vec::<Friend>::new()));

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

    provide_context(user);
    provide_context(friends);

    view! {
        <Router>
            <Routes fallback=move || view!{ "404 Not Found" }>
                <Route path=path!("/") view=LoginPage />
                <Route path=path!("/main") view=MainPage />
                <Route path=path!("/chat/:id") view=Chat />
            </Routes>
        </Router>
    }
}
