use leptos::prelude::*;
use gloo_file::futures::read_as_bytes;
use trainwreck_core::GtfsData;
use crate::state::AppState;

#[component]
pub fn FileUpload() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();

    let on_change = move |ev: web_sys::Event| {
        use wasm_bindgen::JsCast;
        let input = ev.target()
            .expect("change event should have a target")
            .dyn_into::<web_sys::HtmlInputElement>()
            .expect("change target should be a file input");
        let files = input.files().expect("file input should have a FileList");
        let Some(file) = files.get(0) else { return };
        let name = file.name();
        let gloo = gloo_file::File::from(file);

        state.loading_gtfs.set(true);
        state.error.set(None);

        wasm_bindgen_futures::spawn_local(async move {
            match read_as_bytes(&gloo).await {
                Err(e) => {
                    state.error.set(Some(format!("Could not read file: {e:?}")));
                    state.loading_gtfs.set(false);
                }
                Ok(bytes) => match GtfsData::from_bytes(&bytes) {
                    Err(e) => {
                        state.error.set(Some(format!("Invalid GTFS archive: {e}")));
                        state.loading_gtfs.set(false);
                    }
                    Ok(data) => {
                        state.loaded_filename.set(Some(name));
                        state.gtfs.set(Some(data));
                        state.loading_gtfs.set(false);
                    }
                }
            }
        });
    };

    view! {
        <div class="file-upload">
            <label class="file-label">
                <span class="file-icon">"⊞"</span>
                <Show
                    when=move || state.loaded_filename.get().is_some()
                    fallback=|| view! { <span>"Drop GTFS ZIP or click to browse"</span> }
                >
                    {move || {
                        let name = state.loaded_filename.get().unwrap_or_default();
                        let count = state.gtfs.get().map(|g| g.stop_count()).unwrap_or(0);
                        view! { <span>{name}" · "{count}" stops"</span> }
                    }}
                </Show>
                <input type="file" accept=".zip" class="file-input" on:change=on_change />
            </label>
            <Show when=move || state.loading_gtfs.get()>
                <p class="loading-msg">"Parsing GTFS archive…"</p>
            </Show>
        </div>
    }
}
