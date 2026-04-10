use leptos::prelude::*;

use crate::api::ApiStop;

/// One row in the departure board, built from an [`crate::api::ApiDeparture`].
#[derive(Debug, Clone)]
pub struct DepartureRow {
    pub trip_id: String,
    pub stop_id: String,
    /// Seconds since midnight of the service day.
    pub departure_secs: u32,
    pub route_name: String,
    /// GTFS `route_type` integer (for CSS class / label).
    pub route_type: Option<u16>,
    pub destination: String,
    /// `None` = no real-time data; positive = late, negative = early.
    pub delay_secs: Option<i32>,
    pub canceled: bool,
    pub skipped: bool,
    pub platform: Option<String>,
}

/// Reactive application state shared across all components via context.
#[derive(Clone, Copy)]
pub struct AppState {
    /// Current text in the stop search box.
    pub stop_query: RwSignal<String>,
    /// Stops matching the current search query.
    pub matched_stops: RwSignal<Vec<ApiStop>>,
    /// Stop IDs the user has selected.
    pub selected_stop_ids: RwSignal<Vec<String>>,
    /// Departure rows for the selected stops.
    pub departures: RwSignal<Vec<DepartureRow>>,
    /// True while an API request is in flight.
    pub loading: RwSignal<bool>,
    /// Latest error message (`None` when no error).
    pub error: RwSignal<Option<String>>,
}

impl AppState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            stop_query: RwSignal::new(String::new()),
            matched_stops: RwSignal::new(vec![]),
            selected_stop_ids: RwSignal::new(vec![]),
            departures: RwSignal::new(vec![]),
            loading: RwSignal::new(false),
            error: RwSignal::new(None),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
