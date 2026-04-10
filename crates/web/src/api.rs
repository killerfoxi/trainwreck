//! Types mirroring the server's JSON API and async fetch helpers.

use serde::Deserialize;

// ── response types ────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Deserialize)]
pub struct ApiStop {
    pub stop_id: String,
    pub stop_name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ApiDeparture {
    pub trip_id: String,
    pub stop_id: String,
    /// Seconds since midnight of the service day.
    pub departure_secs: u32,
    pub route_name: String,
    pub route_type: Option<u16>,
    pub destination: String,
    /// `None` = no real-time data; positive = late, negative = early.
    pub delay_secs: Option<i32>,
    pub canceled: bool,
    pub skipped: bool,
}

// ── fetch helpers ─────────────────────────────────────────────────────────────

/// Fetch stops matching `query` from the server.
///
/// Calls `GET /api/stops?q=<query>`.
///
/// # Errors
/// Returns a `String` describing the error if the HTTP request or JSON
/// deserialisation fails.
pub async fn fetch_stops(query: &str) -> Result<Vec<ApiStop>, String> {
    let encoded = js_sys::encode_uri_component(query);
    let url = format!("/api/stops?q={encoded}");
    reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .json::<Vec<ApiStop>>()
        .await
        .map_err(|e| e.to_string())
}

/// Fetch departures for the given stop IDs from the server.
///
/// Calls `GET /api/schedule?stop_ids=<id1,id2,...>`. The server resolves the
/// current date, active services, and merges real-time data.
///
/// # Errors
/// Returns a `String` describing the error if the HTTP request or JSON
/// deserialisation fails.
pub async fn fetch_schedule(stop_ids: &[String]) -> Result<Vec<ApiDeparture>, String> {
    let ids = stop_ids.join(",");
    let url = format!("/api/schedule?stop_ids={ids}");
    reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .json::<Vec<ApiDeparture>>()
        .await
        .map_err(|e| e.to_string())
}
