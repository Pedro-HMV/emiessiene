mod app;
mod components;

use components::loginpage_component::LoginPage;
use components::mainpage_component::MainPage;
use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| {
        view! {
            <Router>
                <Routes fallback=move || view!{ "404 Not Found" }>
                    <Route path=path!("/") view=LoginPage />
                    <Route path=path!("/main") view=MainPage />
                </Routes>
            </Router>
        }
    })
}
