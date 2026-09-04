use super::models::XMPP_SERVER;
use crate::app::invoke;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen_futures::spawn_local;

#[derive(Serialize, Deserialize)]
struct XmppRegisterArgs {
    jid: String,
    password: String,
}

#[component]
pub fn RegisterPage() -> impl IntoView {
    let (username, set_username) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (is_loading, set_is_loading) = signal(false);
    let (error_message, set_error_message) = signal(String::new());

    let navigate = use_navigate();

    let handle_register = move |_| {
        set_is_loading.set(true);
        set_error_message.set(String::new());

        let username_value = username.get();
        let password_value = password.get();
        let navigate = navigate.clone();

        // Basic validation
        if username_value.trim().is_empty() || password_value.trim().is_empty() {
            set_error_message.set("All fields are required".to_string());
            set_is_loading.set(false);
            return;
        }

        spawn_local(async move {
            let jid = format!("{}@{}", username_value.trim(), XMPP_SERVER);
            let register_args = XmppRegisterArgs {
                jid: jid.clone(),
                password: password_value.clone(),
            };

            log::info!("Registering XMPP account for {}", register_args.jid);

            let result = invoke("xmpp_register", to_value(&register_args).unwrap()).await;

            match from_value::<serde_json::Value>(result) {
                Ok(resp) if resp["success"].as_bool().unwrap_or(false) => {
                    log::info!("Registration successful for {}", register_args.jid);
                    set_is_loading.set(false);
                    navigate("/", Default::default());
                }
                Ok(resp) => {
                    let msg = resp["error"]
                        .as_str()
                        .unwrap_or("Registration failed")
                        .to_string();
                    log::error!("Registration failed: {}", msg);
                    set_error_message.set(msg);
                    set_is_loading.set(false);
                }
                Err(e) => {
                    log::error!("Failed to parse registration response: {:?}", e);
                    set_error_message.set("Connection error during registration".to_string());
                    set_is_loading.set(false);
                }
            }
        });
    };

    view! {
        <div id="register_container" class="flex-col">
            <div id="register_title">"Create NTO Account"</div>
            <div id="register_subtitle">"Join the nostalgic messaging experience"</div>

            <form id="register_form" class="flex-col">
                <div class="form-group">
                    <label for="username">"Username"</label>
                    <input
                        type="text"
                        id="username"
                        placeholder="Choose a username"
                        value=move || username.get()
                        on:input=move |ev| set_username.set(event_target_value(&ev))
                        disabled=move || is_loading.get()
                    />
                </div>

                <div class="form-group">
                    <label for="password">"Password"</label>
                    <input
                        type="password"
                        id="password"
                        placeholder="Choose a secure password"
                        value=move || password.get()
                        on:input=move |ev| set_password.set(event_target_value(&ev))
                        disabled=move || is_loading.get()
                    />
                </div>

                <Show when=move || !error_message.get().is_empty()>
                    <div class="error-message">
                        {move || error_message.get()}
                    </div>
                </Show>

                <button
                    type="button"
                    on:click=handle_register
                    disabled=move || is_loading.get()
                    class="register-button"
                >
                    {move || if is_loading.get() { "Creating Account..." } else { "Create Account" }}
                </button>

                <div class="login-link">
                    "Already have an account? "
                    <a href="/">"Sign In"</a>
                </div>
            </form>
        </div>
    }
}
