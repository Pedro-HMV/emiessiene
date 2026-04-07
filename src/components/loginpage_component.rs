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
struct SaveLoginPrefsArgs {
    jid: String,
    #[serde(rename = "rememberMe")]
    remember_me: bool,
    #[serde(rename = "autoSignIn")]
    auto_sign_in: bool,
    availability: String,
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

    // Pre-fill form from last remembered login and restore checkbox state
    spawn_local(async move {
        if let Ok(result) = invoke_catching("get_saved_profile", JsValue::null()).await {
            if let Ok(profile) = from_value::<SavedProfile>(result) {
                if !profile.jid.is_empty() {
                    set_username.set(profile.jid);
                }
                set_remember_me.set(profile.remember_me);
                set_auto_sign_in.set(profile.auto_sign_in);
                let avail = match profile.last_availability.as_str() {
                    "Away" => Availability::Away,
                    "Busy" => Availability::Busy,
                    "Offline" => Availability::Offline,
                    _ => Availability::Online,
                };
                set_availability.set(avail);
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

    // Shared sign-in logic extracted so it can be called from button click AND Enter key
    let do_sign_in = {
        let username = username;
        let password = password;
        let availability = availability;
        let set_user = set_user;
        let set_error_message = set_error_message;
        let set_is_loading = set_is_loading;
        let set_should_navigate = set_should_navigate;
        let set_fatal_error = set_fatal_error;
        let remember_me = remember_me;
        let auto_sign_in = auto_sign_in;

        move || {
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

            let jid = if username.get().trim().contains('@') {
                username.get().trim().to_string()
            } else {
                format!("{}@nto.local", username.get().trim())
            };
            let password_value = password.get().clone();
            let availability_value = availability.get().clone();
            let remember_me_captured = remember_me.get();
            let auto_sign_in_captured = auto_sign_in.get();

            let set_user = set_user;
            let set_error_message = set_error_message;
            let set_is_loading = set_is_loading;
            let set_should_navigate = set_should_navigate;
            let set_fatal_error = set_fatal_error;

            spawn_local(async move {
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
                                    log::info!("XMPP connection successful for {}", jid);

                                    // Set user initial state — name defaults to JID local-part;
                                    // the xmpp_vcard_received listener will update it once vCard arrives
                                    let local_part =
                                        jid.split('@').next().unwrap_or(&jid).to_string();
                                    set_user.update(|user| {
                                        user.email = jid.clone();
                                        user.name = local_part;
                                        user.availability = availability_value.clone();
                                        user.flavour_text = String::new();
                                    });

                                    // Save login prefs (JID only stored if remember_me is true)
                                    let avail_str = match availability_value {
                                        Availability::Away => "Away",
                                        Availability::Busy => "Busy",
                                        Availability::Offline => "Offline",
                                        Availability::Online => "Online",
                                    };
                                    if let Ok(args) = to_value(&SaveLoginPrefsArgs {
                                        jid: jid.clone(),
                                        remember_me: remember_me_captured,
                                        auto_sign_in: auto_sign_in_captured,
                                        availability: avail_str.to_string(),
                                    }) {
                                        let _ = invoke_catching("save_login_prefs", args).await;
                                    }

                                    // Listen for vCard response and update user name/flavour_text
                                    {
                                        #[wasm_bindgen]
                                        extern "C" {
                                            #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
                                            async fn listen(
                                                event: &str,
                                                callback: &js_sys::Function,
                                            ) -> JsValue;
                                        }
                                        let set_user_vcard = set_user;
                                        let vcard_cb = wasm_bindgen::closure::Closure::wrap(
                                            Box::new(move |raw: JsValue| {
                                                if let Ok(envelope) =
                                                    from_value::<serde_json::Value>(raw)
                                                {
                                                    let payload = &envelope["payload"];
                                                    let nickname = payload["nickname"]
                                                        .as_str()
                                                        .unwrap_or("")
                                                        .to_string();
                                                    let ft = payload["flavour_text"]
                                                        .as_str()
                                                        .unwrap_or("")
                                                        .to_string();
                                                    set_user_vcard.update(|u| {
                                                        if !nickname.is_empty() {
                                                            u.name = nickname;
                                                        }
                                                        u.flavour_text = ft;
                                                    });
                                                }
                                            })
                                                as Box<dyn Fn(JsValue)>,
                                        );
                                        spawn_local(async move {
                                            let _ = listen(
                                                "xmpp_vcard_received",
                                                vcard_cb.as_ref().unchecked_ref(),
                                            )
                                            .await;
                                            vcard_cb.forget();
                                        });
                                    }

                                    // Request vCard from server
                                    let _ = invoke_catching(
                                        "xmpp_fetch_vcard",
                                        to_value(&serde_json::json!({})).unwrap(),
                                    )
                                    .await;

                                    set_is_loading.set(false);
                                    set_should_navigate.set(true);
                                } else {
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

    let handle_sign_in = {
        let do_sign_in = do_sign_in.clone();
        move |ev: leptos::ev::MouseEvent| {
            ev.prevent_default();
            do_sign_in();
        }
    };

    let on_enter_username = {
        let do_sign_in = do_sign_in.clone();
        move |ev: leptos::ev::KeyboardEvent| {
            if ev.key() == "Enter" {
                ev.prevent_default();
                do_sign_in();
            }
        }
    };

    let on_enter_password = {
        let do_sign_in = do_sign_in.clone();
        move |ev: leptos::ev::KeyboardEvent| {
            if ev.key() == "Enter" {
                ev.prevent_default();
                do_sign_in();
            }
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
        <div id="login_container">
            <div id="login_title">"NTO"</div>
            <div id="login_avatar">
                <div id="login_avatar_img" style="width: 150px; height: 150px; background: black;">
                    a
                </div>
            </div>
            <form id="login_form">
                <input
                    type="text"
                    id="login_username"
                    placeholder="Email (e.g., pedro@hotmail.com)"
                    prop:value=username
                    on:input=move |ev| set_username.set(event_target_value(&ev))
                    on:keydown=on_enter_username
                />
                <input
                    type="password"
                    id="login_password"
                    placeholder="Password"
                    prop:value=password
                    on:input=move |ev| set_password.set(event_target_value(&ev))
                    on:keydown=on_enter_password
                />

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
                    "Status: "
                    <select
                        id="login_availability"
                        prop:value=move || match availability.get() {
                            Availability::Away => "Away",
                            Availability::Busy => "Busy",
                            Availability::Offline => "Offline",
                            Availability::Online => "Online",
                        }
                        on:change=update_availability
                    >
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
                            prop:checked=remember_me
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
                            prop:checked=auto_sign_in
                            on:change=move |ev| set_auto_sign_in.set(event_target_checked(&ev))
                        />
                        "Sign me in automatically"
                    </label>
                </div>
                <button
                    type="button"
                    id="login_submit"
                    on:click=handle_sign_in
                    disabled=move || is_loading.get()
                >
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
