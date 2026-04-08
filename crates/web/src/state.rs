use leptos::prelude::*;
use trainwreck_core::GtfsData;
use trainwreck_core::realtime::model::{DepartureStatus, RealtimeFeed};
use trainwreck_core::gtfs::model::Stop;

/// One row in the departure board.
#[derive(Debug, Clone)]
pub struct DepartureRow {
    pub trip_id:        String,
    pub stop_id:        String,
    /// Raw seconds since midnight (use `time::format_hhmm` to display).
    pub departure_secs: u32,
    pub route_name:     String,
    /// GTFS route_type integer (for CSS class / label).
    pub route_type:     Option<u16>,
    pub destination:    String,
    /// `None` means real-time not available for this trip.
    pub status:         Option<DepartureStatus>,
}

#[derive(Clone, Copy)]
pub struct AppState {
    /// Loaded GTFS data (None until user uploads a ZIP).
    pub gtfs:              RwSignal<Option<GtfsData>>,
    /// File name of the loaded ZIP (for display).
    pub loaded_filename:   RwSignal<Option<String>>,
    /// Real-time API key (persisted to localStorage).
    pub api_key:           RwSignal<String>,
    /// Current text in the stop search box.
    pub stop_query:        RwSignal<String>,
    /// Stops matching the current query.
    pub matched_stops:     RwSignal<Vec<Stop>>,
    /// Stop IDs the user has selected (usually 1, sometimes a group).
    pub selected_stop_ids: RwSignal<Vec<String>>,
    /// Computed departure rows (updated when stop selection changes).
    pub departures:        RwSignal<Vec<DepartureRow>>,
    /// Latest real-time feed (None if no key / fetch failed).
    pub realtime:          RwSignal<Option<RealtimeFeed>>,
    pub loading_gtfs:      RwSignal<bool>,
    pub loading_realtime:  RwSignal<bool>,
    pub error:             RwSignal<Option<String>>,
}

impl AppState {
    pub fn new() -> Self {
        // Restore api_key from localStorage if available.
        let stored_key = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item("otd_api_key").ok().flatten())
            .unwrap_or_default();

        Self {
            gtfs:              RwSignal::new(None),
            loaded_filename:   RwSignal::new(None),
            api_key:           RwSignal::new(stored_key),
            stop_query:        RwSignal::new(String::new()),
            matched_stops:     RwSignal::new(vec![]),
            selected_stop_ids: RwSignal::new(vec![]),
            departures:        RwSignal::new(vec![]),
            realtime:          RwSignal::new(None),
            loading_gtfs:      RwSignal::new(false),
            loading_realtime:  RwSignal::new(false),
            error:             RwSignal::new(None),
        }
    }
}
