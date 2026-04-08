use leptos::prelude::*;
use crate::state::AppState;

#[component]
pub fn ApiKeyInput() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();

    let on_input = move |ev: web_sys::Event| {
        use wasm_bindgen::JsCast;
        let val = ev.target().unwrap()
            .dyn_into::<web_sys::HtmlInputElement>().unwrap()
            .value();
        // Persist to localStorage
        if let Some(storage) = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
        {
            let _ = storage.set_item("otd_api_key", &val);
        }
        state.api_key.set(val);
    };

    view! {
        <div class="api-key-section">
            <label class="field-label">"Real-time API key (optional)"</label>
            <input
                type="password"
                class="api-key-input"
                placeholder="OTD Bearer token"
                prop:value=move || state.api_key.get()
                on:input=on_input
            />
            <p class="field-note">
                "Get a key at "
                <a href="https://opentransportdata.swiss" target="_blank">"opentransportdata.swiss"</a>
                ". Requires CORS — see notes below."
            </p>
        </div>
    }
}
