use gloo_timers::future::TimeoutFuture;
use leptos::prelude::*;

use crate::api::{ApiDeparture, ApiStop, fetch_schedule, fetch_stops};
use crate::state::{AppState, DepartureRow};
use crate::time;

/// Build a sorted list of upcoming [`DepartureRow`]s from the API response,
/// keeping only departures within 1 minute ago and up to 3 hours ahead.
pub(crate) fn build_rows(deps: Vec<ApiDeparture>) -> Vec<DepartureRow> {
    let now = i64::from(time::now_secs_since_midnight());
    let mut rows: Vec<DepartureRow> = deps
        .into_iter()
        .filter(|d| {
            let dep = i64::from(d.departure_secs);
            dep >= now - 60 && dep <= now + 10_800
        })
        .map(|d| DepartureRow {
            trip_id: d.trip_id,
            stop_id: d.stop_id,
            departure_secs: d.departure_secs,
            route_name: d.route_name,
            route_type: d.route_type,
            destination: d.destination,
            delay_secs: d.delay_secs,
            canceled: d.canceled,
            skipped: d.skipped,
            platform: d.platform,
        })
        .collect();
    rows.sort_unstable_by_key(|r| r.departure_secs);
    rows
}

#[component]
pub fn StopSearch() -> impl IntoView {
    let state = use_context::<AppState>().expect("AppState must be provided");

    // Generation counter to debounce: stale closures skip if a newer input arrived.
    let generation = RwSignal::new(0u32);
    let searching = RwSignal::new(false);

    let on_input = move |ev: web_sys::Event| {
        use wasm_bindgen::JsCast;

        let Some(target) = ev.target() else { return };
        let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() else { return };
        let val = input.value();

        state.stop_query.set(val.clone());

        if val.is_empty() {
            state.matched_stops.set(vec![]);
            return;
        }

        let this_gen = generation.get() + 1;
        generation.set(this_gen);

        wasm_bindgen_futures::spawn_local(async move {
            TimeoutFuture::new(300).await;
            // Stale check: if generation advanced, a newer input arrived — skip.
            if generation.get() != this_gen {
                return;
            }
            searching.set(true);
            match fetch_stops(&val).await {
                Ok(stops) => state.matched_stops.set(stops),
                Err(e) => state.error.set(Some(format!("Stop search failed: {e}"))),
            }
            searching.set(false);
        });
    };

    let select_stop = move |stop: ApiStop| {
        let stop_ids = vec![stop.stop_id.clone()];
        state.selected_stop_ids.set(stop_ids.clone());
        state.stop_query.set(stop.stop_name.clone());
        state.matched_stops.set(vec![]);
        state.loading.set(true);
        state.error.set(None);

        wasm_bindgen_futures::spawn_local(async move {
            match fetch_schedule(&stop_ids).await {
                Ok(deps) => state.departures.set(build_rows(deps)),
                Err(e) => state.error.set(Some(format!("Schedule fetch failed: {e}"))),
            }
            state.loading.set(false);
        });
    };

    view! {
        <div class="stop-search">
            <input
                type="search"
                class="search-input"
                placeholder="Search stops…"
                prop:value=move || state.stop_query.get()
                on:input=on_input
            />
            <Show
                when=move || searching.get()
                fallback=move || view! {
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
                }
            >
                {skeleton_stops()}
            </Show>
        </div>
    }
}

fn skeleton_stops() -> impl IntoView {
    // Vary name widths so the skeleton looks like real content.
    let name_widths = ["70%", "85%", "60%", "75%"];
    view! {
        <ul class="stop-list">
            {name_widths.iter().map(|&w| view! {
                <li class="stop-item">
                    <div class="skeleton skel-stop-name" style={format!("width:{w}")}></div>
                    <div class="skeleton skel-stop-id"></div>
                </li>
            }).collect::<Vec<_>>()}
        </ul>
    }
}
