use super::models::Availability;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

#[component]
pub fn LoginPage() -> impl IntoView {
    let (_, set_availability) = signal(Availability::Online);
    let (remember_me, set_remember_me) = signal(false);
    let (auto_sign_in, set_auto_sign_in) = signal(false);
    let navigate = use_navigate();

    let update_availability = move |ev| {
        let value = event_target_value(&ev);
        set_availability.set(match value.as_str() {
            "Away" => Availability::Away,
            "Busy" => Availability::Busy,
            "Offline" => Availability::Offline,
            _ => Availability::Online,
        });
    };

    let sign_in = move |ev: leptos::ev::MouseEvent| {
        ev.prevent_default();
        navigate("/main", Default::default());
    };

    view! {
        <div id="login_container" class="flex-col">
            <div id="login_title">"EmiEssiEne"</div>
            <div id="login_avatar">
                <div id="login_avatar_img" style="width: 150px; height: 150px; background: black;">
                    a
                </div>
            </div>
            <form id="login_form" class="flex-col">
                <input type="text" id="login_username" placeholder="Username" />
                <input type="password" id="login_password" placeholder="Password" />
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
                <button type="button" on:click=sign_in>
                    "Sign In"
                </button>
            </form>
        </div>
    }
}
