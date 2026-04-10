use leptos::prelude::*;

use crate::api::fetch_schedule;
use crate::state::{AppState, DepartureRow};
use crate::time;
use crate::components::stop_search::build_rows;

#[component]
pub fn DepartureBoard() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();

    let refresh = move |_| {
        let ids = state.selected_stop_ids.get();
        if ids.is_empty() {
            return;
        }
        state.loading.set(true);
        state.error.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_schedule(&ids).await {
                Ok(deps) => state.departures.set(build_rows(deps)),
                Err(e) => state.error.set(Some(format!("Refresh failed: {e}"))),
            }
            state.loading.set(false);
        });
    };

    view! {
        <div class="board-area">
            <Show
                when=move || !state.selected_stop_ids.get().is_empty()
                fallback=|| view! {
                    <div class="empty-state">
                        <p>"Search for a stop to see departures."</p>
                    </div>
                }
            >
                <div class="board-header">
                    <h2>{move || state.stop_query.get()}</h2>
                    <div class="board-controls">
                        <Show when=move || state.loading.get()>
                            <span class="loading-msg">"⟳"</span>
                        </Show>
                        <button class="btn-refresh" on:click=refresh>"Refresh"</button>
                    </div>
                </div>

                <Show
                    when=move || !state.departures.get().is_empty()
                    fallback=|| view! { <p class="no-deps">"No departures found."</p> }
                >
                    <div class="departure-list">
                        <For
                            each=move || state.departures.get()
                            key=|r| format!("{}-{}", r.trip_id, r.departure_secs)
                            children=move |row| {
                                let now = time::now_secs_since_midnight();
                                let hhmm = time::format_hhmm(row.departure_secs);
                                let rel = time::relative_time(row.departure_secs, now);
                                let css = trainwreck_core::gtfs::model::Route::transport_css_class_for(row.route_type);
                                let (status_text, status_class) = departure_status(&row);
                                view! {
                                    <span class="dep-time">{hhmm}</span>
                                    <span class="dep-rel">{rel}</span>
                                    <span class={format!("route-badge {css}")}>{row.route_name}</span>
                                    <span class="dep-dest">{row.destination}</span>
                                    <span class={format!("status-badge {status_class}")}>{status_text}</span>
                                }
                            }
                        />
                    </div>
                </Show>
            </Show>

            <Show when=move || state.error.get().is_some()>
                <p class="error-msg">{move || state.error.get().unwrap_or_default()}</p>
            </Show>
        </div>
    }
}

fn departure_status(row: &DepartureRow) -> (String, &'static str) {
    if row.canceled {
        return ("CANCELLED".into(), "cancelled");
    }
    if row.skipped {
        return ("SKIPPED".into(), "cancelled");
    }
    match row.delay_secs {
        None => (String::new(), ""),
        Some(0) => ("on time".into(), "on-time"),
        Some(d) if d > 0 => (format!("+{}m", d / 60), "delayed"),
        Some(d) => (format!("{}m", d / 60), "early"),
    }
}
