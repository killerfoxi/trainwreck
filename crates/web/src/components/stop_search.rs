use leptos::prelude::*;
use gloo_timers::future::TimeoutFuture;
use trainwreck_core::gtfs::model::Stop;
use crate::state::{AppState, DepartureRow};

/// Recomputes `state.departures` for the currently selected stops.
/// Call this after changing `state.selected_stop_ids` or after a realtime refresh.
pub fn compute_departures(state: &AppState) {
    let Some(gtfs) = state.gtfs.get() else { return };
    let ids = state.selected_stop_ids.get();
    if ids.is_empty() { return; }

    let stop_ids: Vec<&str> = ids.iter().map(|s| s.as_str()).collect();
    let (y, m, d) = crate::time::today_ymd();
    let date = jiff::civil::Date::new(y as i16, m as i8, d as i8).unwrap();
    let active = gtfs.active_service_ids(date);
    let schedule = gtfs.schedule_for_stops(&stop_ids, active.as_ref());

    let now_secs = crate::time::now_secs_since_midnight();
    let realtime = state.realtime.get();

    let mut rows: Vec<DepartureRow> = schedule
        .departures()
        .into_iter()
        .filter(|(st, _, _)| {
            // Keep departures within last 1 min and up to 3h ahead
            let dep = st.departure_time.as_secs() as i64;
            let now = now_secs as i64;
            dep >= now - 60 && dep <= now + 10800
        })
        .map(|(st, trip, route)| {
            let route_name = route
                .and_then(|r| r.route_short_name.as_deref().or(r.route_long_name.as_deref()))
                .unwrap_or(trip.route_id.as_str())
                .to_string();
            let route_type = route.and_then(|r| r.route_type);
            let destination = trip.trip_headsign.clone().unwrap_or_else(|| "?".into());
            let status = realtime.as_ref()
                .and_then(|f| f.status_for(&trip.trip_id, &st.stop_id));
            DepartureRow {
                trip_id: trip.trip_id.clone(),
                stop_id: st.stop_id.clone(),
                departure_secs: st.departure_time.as_secs(),
                route_name,
                route_type,
                destination,
                status,
            }
        })
        .collect();

    rows.sort_unstable_by_key(|r| r.departure_secs);
    state.departures.set(rows);
}

#[component]
pub fn StopSearch() -> impl IntoView {
    let state = use_context::<AppState>().unwrap();

    // Generation counter to debounce: stale closures skip if a newer input arrived
    let generation = RwSignal::new(0u32);

    let on_input = move |ev: web_sys::Event| {
        use wasm_bindgen::JsCast;
        let val = ev.target().unwrap()
            .dyn_into::<web_sys::HtmlInputElement>().unwrap()
            .value();
        state.stop_query.set(val.clone());

        let this_gen = generation.get() + 1;
        generation.set(this_gen);

        wasm_bindgen_futures::spawn_local(async move {
            TimeoutFuture::new(300).await;
            // Stale check: if generation advanced, a newer input arrived — skip
            if generation.get() != this_gen { return; }
            let results = state.gtfs.get()
                .map(|g| g.find_stops(&val).into_iter().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            state.matched_stops.set(results);
        });
    };

    let select_stop = move |stop: Stop| {
        state.selected_stop_ids.set(vec![stop.stop_id.clone()]);
        state.stop_query.set(stop.stop_name.clone());
        state.matched_stops.set(vec![]);
        compute_departures(&state);

        // Fetch real-time if key is set
        let key = state.api_key.get();
        if !key.is_empty() {
            wasm_bindgen_futures::spawn_local(async move {
                state.loading_realtime.set(true);
                match trainwreck_core::fetch_trip_updates(&key).await {
                    Ok(feed) => {
                        state.realtime.set(Some(feed));
                        compute_departures(&state);
                    }
                    Err(_) => {
                        // Real-time failure is non-fatal; static schedule remains
                    }
                }
                state.loading_realtime.set(false);
            });
        }
    };

    view! {
        <div class="stop-search">
            <Show when=move || state.gtfs.get().is_some()>
                <input
                    type="search"
                    class="search-input"
                    placeholder="Search stops…"
                    prop:value=move || state.stop_query.get()
                    on:input=on_input
                />
                <ul class="stop-list">
                    <For
                        each=move || state.matched_stops.get()
                        key=|s| s.stop_id.clone()
                        children=move |stop| {
                            let stop_clone = stop.clone();
                            let is_selected = {
                                let id = stop.stop_id.clone();
                                move || state.selected_stop_ids.get().contains(&id)
                            };
                            view! {
                                <li
                                    class="stop-item"
                                    class:selected=is_selected
                                    on:click=move |_| select_stop(stop_clone.clone())
                                >
                                    <span class="stop-name">{stop.stop_name.clone()}</span>
                                    <span class="stop-id">{stop.stop_id.clone()}</span>
                                </li>
                            }
                        }
                    />
                </ul>
            </Show>
        </div>
    }
}
