use super::models::{Availability, SavedProfile, User};
use leptos::prelude::*;
use leptos::web_sys;
use leptos_router::hooks::use_navigate;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "tauri"], js_name = "invoke", catch)]
    async fn invoke_catching(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;
}

// Struct for XMPP connection arguments
#[derive(Serialize, Deserialize)]
struct XmppConnectArgs {
    jid: String,
    password: String,
}

#[derive(Serialize)]
struct SaveJidArgs {
    jid: String,
}

#[component]
pub fn LoginPage() -> impl IntoView {
    let (username, set_username) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (availability, set_availability) = signal(Availability::Online);
    let (remember_me, set_remember_me) = signal(false);
    let (auto_sign_in, set_auto_sign_in) = signal(false);
    let (is_loading, set_is_loading) = signal(false);
    let (error_message, set_error_message) = signal(String::new());
    let (fatal_error, set_fatal_error) = signal(Option::<String>::None);
    let navigate = use_navigate();

    // Get the global user setter from context
    let set_user = use_context::<WriteSignal<User>>().expect("No user setter context");

    // Create a signal to trigger navigation
    let (should_navigate, set_should_navigate) = signal(false);

    // Create an effect that handles navigation
    Effect::new({
        let navigate = navigate.clone();
        move |_| {
            if should_navigate.get() {
                navigate("/main", Default::default());
            }
        }
    });

    // Pre-fill JID from last remembered login (only needs the jid field)
    spawn_local(async move {
        if let Ok(result) = invoke_catching("get_saved_profile", JsValue::null()).await {
            if let Ok(profile) = from_value::<SavedProfile>(result) {
                if !profile.jid.is_empty() {
                    set_username.set(profile.jid);
                }
            }
        }
    });

    let update_availability = move |ev| {
        let value = event_target_value(&ev);
        set_availability.set(match value.as_str() {
            "Away" => Availability::Away,
            "Busy" => Availability::Busy,
            "Offline" => Availability::Offline,
            _ => Availability::Online,
        });
    };

    let handle_sign_in = {
        let username = username;
        let password = password;
        let availability = availability;
        let set_user = set_user;
        let set_error_message = set_error_message;
        let set_is_loading = set_is_loading;
        let set_should_navigate = set_should_navigate;

        move |ev: leptos::ev::MouseEvent| {
            ev.prevent_default();

            // Clear any previous error messages
            set_error_message.set(String::new());

            // Validate input
            if username.get().trim().is_empty() {
                set_error_message.set("Email is required".to_string());
                return;
            }

            if password.get().trim().is_empty() {
                set_error_message.set("Password is required".to_string());
                return;
            }

            // Set loading state
            set_is_loading.set(true);

            // Clone values for the async block
            let jid = if username.get().trim().contains('@') {
                // If already an email, use as-is
                username.get().trim().to_string()
            } else {
                // If just username, append domain
                format!("{}@nto.local", username.get().trim())
            };
            let password_value = password.get().clone();
            let availability_value = availability.get().clone();
            let _username_value = username.get().clone();

            // Clone the signals for the async block
            let set_user = set_user;
            let set_error_message = set_error_message;
            let set_is_loading = set_is_loading;
            let set_should_navigate = set_should_navigate;
            let set_fatal_error = set_fatal_error;
            let remember_me_captured = remember_me.get();

            // Spawn async task for XMPP connection
            spawn_local(async move {
                // Create arguments for Tauri command using struct
                let connect_args = XmppConnectArgs {
                    jid: jid.clone(),
                    password: password_value.clone(),
                };
                let connect_js_args = match to_value(&connect_args) {
                    Ok(v) => v,
                    Err(e) => {
                        set_fatal_error.set(Some(format!("Serialization error: {:?}", e)));
                        set_is_loading.set(false);
                        return;
                    }
                };

                match invoke_catching("xmpp_connect", connect_js_args).await {
                    Ok(result) => {
                        match from_value::<serde_json::Value>(result) {
                            Ok(response) => {
                                if response["success"].as_bool().unwrap_or(false) {
                                    // XMPP connection successful
                                    log::info!("XMPP connection successful for {}", jid);

                                    // Load saved profile now (inside the same async task — no race condition)
                                    let profile =
                                        invoke_catching("get_saved_profile", JsValue::null())
                                            .await
                                            .ok()
                                            .and_then(|v| from_value::<SavedProfile>(v).ok());

                                    let display_name = profile
                                        .as_ref()
                                        .filter(|p| !p.nickname.is_empty())
                                        .map(|p| p.nickname.clone())
                                        .unwrap_or_else(|| {
                                            jid.split('@').next().unwrap_or(&jid).to_string()
                                        });

                                    let restored_ft =
                                        profile.map(|p| p.flavour_text).unwrap_or_default();

                                    // Update global user context
                                    set_user.update(|user| {
                                        user.name = display_name;
                                        user.availability = availability_value.clone();
                                        user.flavour_text = restored_ft;
                                    });

                                    // Save JID for remember me
                                    if remember_me_captured {
                                        if let Ok(args) =
                                            to_value(&SaveJidArgs { jid: jid.clone() })
                                        {
                                            let _ = invoke_catching("save_jid", args).await;
                                        }
                                    }

                                    // Navigate to main page
                                    // (initial presence is sent automatically by the backend on Event::Online)
                                    set_is_loading.set(false);
                                    set_should_navigate.set(true);
                                } else {
                                    // XMPP connection failed
                                    let error_msg =
                                        response["error"].as_str().unwrap_or("Unknown error");
                                    log::error!("XMPP connection failed: {}", error_msg);
                                    set_error_message.set(
                                        "Login failed. Please check your credentials.".to_string(),
                                    );
                                    set_is_loading.set(false);
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to parse XMPP response: {:?}", e);
                                set_error_message
                                    .set("Login failed. Connection error.".to_string());
                                set_is_loading.set(false);
                            }
                        }
                    }
                    Err(e) => {
                        let msg = e.as_string().unwrap_or_else(|| format!("{:?}", e));
                        log::error!("xmpp_connect threw: {}", msg);
                        set_fatal_error.set(Some(msg));
                        set_is_loading.set(false);
                    }
                }
            });
        }
    };

    view! {
        <Show when=move || fatal_error.get().is_some()>
            <div style="position:fixed;inset:0;display:flex;align-items:center;justify-content:center;background:rgba(0,0,0,0.6);z-index:9999;">
                <div style="background:white;padding:24px;border:2px solid #c00;min-width:300px;max-width:420px;text-align:center;">
                    <div style="font-weight:bold;font-size:16px;margin-bottom:12px;color:#c00;">
                        "Error"
                    </div>
                    <div style="margin-bottom:20px;font-family:monospace;font-size:13px;word-break:break-all;">
                        {move || fatal_error.get().unwrap_or_default()}
                    </div>
                    <button
                        style="padding:6px 24px;"
                        on:click=move |_| {
                            if let Some(window) = web_sys::window() {
                                let _ = window.location().reload();
                            }
                        }
                    >
                        "Ok"
                    </button>
                </div>
            </div>
        </Show>
        <div id="login_container" class="flex-col">
            <div id="login_title">"NTO"</div>
            <div id="login_avatar">
                <div id="login_avatar_img" style="width: 150px; height: 150px; background: black;">
                    a
                </div>
            </div>
            <form id="login_form" class="flex-col">
                <input
                    type="text"
                    id="login_username"
                    placeholder="Email (e.g., pedro@hotmail.com)"
                    prop:value=username
                    on:input=move |ev| set_username.set(event_target_value(&ev))
                />
                <input
                    type="password"
                    id="login_password"
                    placeholder="Password"
                    prop:value=password
                    on:input=move |ev| set_password.set(event_target_value(&ev))
                />

                // Show error message if any
                <div class="error-container" style="min-height: 20px; margin: 10px 0;">
                    {move || {
                        let error = error_message.get();
                        let display_style = if error.is_empty() {
                            "color: red; display: none;"
                        } else {
                            "color: red;"
                        };
                        let content = if error.is_empty() { String::new() } else { error };

                        view! {
                            <div class="error-message" style=display_style>
                                {content}
                            </div>
                        }
                    }}
                </div>

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
                <button type="button" on:click=handle_sign_in disabled=move || is_loading.get()>
                    {move || if is_loading.get() { "Connecting..." } else { "Sign In" }}
                </button>

                <div class="register-link" style="margin-top: 20px; text-align: center;">
                    "Don't have an account? "
                    <a href="/register" style="color: #0066cc; text-decoration: none;">
                        "Create Account"
                    </a>
                </div>
            </form>
        </div>
    }
}
