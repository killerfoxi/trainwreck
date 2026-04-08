use leptos::prelude::*;

#[component]
pub fn Header() -> impl IntoView {
    view! {
        <header class="header">
            <h1>"trainwreck"</h1>
            <span class="tagline">"transit departures"</span>
        </header>
    }
}
