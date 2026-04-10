pub mod api;
mod components;
pub mod state;
pub mod time;

use leptos::mount::mount_to_body;
use leptos::prelude::*;
use state::AppState;
use components::{
    header::Header,
    stop_search::StopSearch,
    departure_board::DepartureBoard,
};

#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    provide_context(AppState::new());
    view! {
        <div class="app">
            <Header/>
            <div class="main">
                <aside class="sidebar">
                    <StopSearch/>
                </aside>
                <DepartureBoard/>
            </div>
        </div>
    }
}
