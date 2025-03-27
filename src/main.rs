mod app;
mod components;

use app::LoginPage;
use components::mainpage_component::MainPage;
use leptos::*;
use leptos_router::*;

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| {
        view! {
            <Router>
                <Routes>
                    <Route path="/" view=LoginPage />
                    <Route path="/main" view=MainPage />
                </Routes>
            </Router>
        }
    })
}
