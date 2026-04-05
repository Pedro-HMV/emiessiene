use super::models::{Availability, User};
use crate::app::invoke;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

// Struct for user registration arguments
#[derive(Serialize, Deserialize)]
struct CreateUserArgs {
    name: String,
    email: String,
    password: String,
    status: String,
    availability: String,
}

#[component]
pub fn RegisterPage() -> impl IntoView {
    let (name, set_name) = signal(String::new());
    let (email, set_email) = signal(String::new());
    let (password, set_password) = signal(String::new());
    let (status, set_status) = signal("Hello from NTO!".to_string());
    let (availability, set_availability) = signal(Availability::Online);
    let (is_loading, set_is_loading) = signal(false);
    let (error_message, set_error_message) = signal(String::new());

    let navigate = use_navigate();

    let handle_register = move |_| {
        set_is_loading.set(true);
        set_error_message.set(String::new());

        let name_value = name.get();
        let email_value = email.get();
        let password_value = password.get();
        let status_value = status.get();
        let availability_value = availability.get();
        let navigate = navigate.clone();

        // Basic validation
        if name_value.trim().is_empty()
            || email_value.trim().is_empty()
            || password_value.trim().is_empty()
        {
            set_error_message.set("All fields are required".to_string());
            set_is_loading.set(false);
            return;
        }

        if !email_value.contains('@') {
            set_error_message.set("Please enter a valid email address".to_string());
            set_is_loading.set(false);
            return;
        }

        spawn_local(async move {
            // Create user registration arguments
            let register_args = CreateUserArgs {
                name: name_value.trim().to_string(),
                email: email_value.trim().to_string(),
                password: password_value.clone(),
                status: status_value.clone(),
                availability: format!("{:?}", availability_value),
            };

            // For now, we'll just simulate user creation and navigate to login
            // In a real implementation, this would call a create_user command
            log::info!(
                "Creating user: {} <{}>",
                register_args.name,
                register_args.email
            );

            // Simulate a brief delay using gloo_timers
            gloo_timers::future::TimeoutFuture::new(1000).await;

            set_is_loading.set(false);

            // Navigate to login page with success message
            navigate("/", Default::default());
        });
    };

    view! {
        <div id="register_container" class="flex-col">
            <div id="register_title">"Create NTO Account"</div>
            <div id="register_subtitle">"Join the nostalgic messaging experience"</div>

            <form id="register_form" class="flex-col">
                <div class="form-group">
                    <label for="name">"Full Name"</label>
                    <input
                        type="text"
                        id="name"
                        placeholder="Enter your full name"
                        value=move || name.get()
                        on:input=move |ev| set_name.set(event_target_value(&ev))
                        disabled=move || is_loading.get()
                    />
                </div>

                <div class="form-group">
                    <label for="email">"Email Address"</label>
                    <input
                        type="email"
                        id="email"
                        placeholder="your.email@domain.com"
                        value=move || email.get()
                        on:input=move |ev| set_email.set(event_target_value(&ev))
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

                <div class="form-group">
                    <label for="status">"Status Message"</label>
                    <input
                        type="text"
                        id="status"
                        placeholder="What's on your mind?"
                        value=move || status.get()
                        on:input=move |ev| set_status.set(event_target_value(&ev))
                        disabled=move || is_loading.get()
                    />
                </div>

                <div class="form-group">
                    <label for="availability">"Initial Status"</label>
                    <select
                        id="availability"
                        on:change=move |ev| {
                            let value = event_target_value(&ev);
                            let avail = match value.as_str() {
                                "Away" => Availability::Away,
                                "Busy" => Availability::Busy,
                                "Offline" => Availability::Offline,
                                _ => Availability::Online,
                            };
                            set_availability.set(avail);
                        }
                        disabled=move || is_loading.get()
                    >
                        <option value="Online" selected=move || matches!(availability.get(), Availability::Online)>
                            "🟢 Online"
                        </option>
                        <option value="Away" selected=move || matches!(availability.get(), Availability::Away)>
                            "🟡 Away"
                        </option>
                        <option value="Busy" selected=move || matches!(availability.get(), Availability::Busy)>
                            "🔴 Busy"
                        </option>
                        <option value="Offline" selected=move || matches!(availability.get(), Availability::Offline)>
                            "⚫ Offline"
                        </option>
                    </select>
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
