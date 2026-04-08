use leptos::prelude::*;
use trainwreck_core::realtime::model::DepartureStatus;
use crate::state::AppState;
use crate::time;

#[component]
pub fn DepartureBoard() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();

    let refresh = move |_: web_sys::MouseEvent| {
        let key = state.api_key.get();
        if key.is_empty() { return; }
        wasm_bindgen_futures::spawn_local(async move {
            state.loading_realtime.set(true);
            if let Ok(feed) = trainwreck_core::fetch_trip_updates(&key).await {
                state.realtime.set(Some(feed));
                crate::components::stop_search::compute_departures(&state);
            }
            state.loading_realtime.set(false);
        });
    };

    view! {
        <div class="board-area">
            <Show
                when=move || !state.selected_stop_ids.get().is_empty()
                fallback=|| view! {
                    <div class="empty-state">
                        <p>"Load a GTFS ZIP and search for a stop."</p>
                    </div>
                }
            >
                <div class="board-header">
                    <h2>{move || {
                        let ids = state.selected_stop_ids.get();
                        state.gtfs.get()
                            .and_then(|g| {
                                g.stops.iter()
                                    .find(|s| ids.contains(&s.stop_id))
                                    .map(|s| s.stop_name.clone())
                            })
                            .unwrap_or_else(|| ids.join(", "))
                    }}</h2>
                    <div class="board-controls">
                        <Show when=move || state.loading_realtime.get()>
                            <span class="loading-msg">"⟳"</span>
                        </Show>
                        <Show when=move || !state.api_key.get().is_empty()>
                            <button class="btn-refresh" on:click=refresh>"Refresh"</button>
                        </Show>
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
                                let rel  = time::relative_time(row.departure_secs, now);
                                let css = trainwreck_core::gtfs::model::Route::transport_css_class_for(row.route_type);
                                let (status_text, status_class) = match row.status {
                                    None => (String::new(), ""),
                                    Some(DepartureStatus::OnTime { delay_secs: 0 }) =>
                                        ("on time".into(), "on-time"),
                                    Some(DepartureStatus::OnTime { delay_secs: d }) if d > 0 =>
                                        (format!("+{}m", d / 60), "delayed"),
                                    Some(DepartureStatus::OnTime { delay_secs: d }) =>
                                        (format!("{}m", d / 60), "early"),
                                    Some(DepartureStatus::Canceled) =>
                                        ("CANCELLED".into(), "cancelled"),
                                    Some(DepartureStatus::Skipped) =>
                                        ("SKIPPED".into(), "cancelled"),
                                };
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
