//! Types mirroring the server's JSON API and async fetch helpers.

use std::sync::OnceLock;

use serde::Deserialize;
use url::Url;

// ── base URL ──────────────────────────────────────────────────────────────────

static BASE_URL: OnceLock<Url> = OnceLock::new();

/// Initialise the base URL from `window.location.origin`.
///
/// Must be called once at application startup before any fetch helper is used.
///
/// # Errors
/// Returns a `String` if the origin cannot be read from the DOM or if the
/// resulting string is not a valid URL.
pub fn init_base_url() -> Result<(), String> {
    let origin = web_sys::window()
        .ok_or_else(|| "no window".to_owned())?
        .location()
        .origin()
        .map_err(|e| format!("window.location.origin unavailable: {e:?}"))?;
    let url = Url::parse(&origin).map_err(|e| format!("invalid origin URL `{origin}`: {e}"))?;
    // Ignore the error if another thread already set it (shouldn't happen in WASM).
    let _ = BASE_URL.set(url);
    Ok(())
}

fn base_url() -> Result<&'static Url, String> {
    BASE_URL.get().ok_or_else(|| "base URL not initialised".to_owned())
}

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
    pub platform: Option<String>,
}

// ── fetch helpers ─────────────────────────────────────────────────────────────

/// Format a reqwest error with its full source chain.
///
/// `reqwest::Error::to_string()` only shows the top-level kind (e.g.
/// "builder error") and drops the underlying cause.  Walking the chain gives
/// the user something actionable.
fn fmt_err(e: &reqwest::Error) -> String {
    use std::error::Error as _;
    let mut msg = e.to_string();
    let mut source = e.source();
    while let Some(cause) = source {
        msg.push_str(": ");
        msg.push_str(&cause.to_string());
        source = cause.source();
    }
    msg
}

/// Fetch stops matching `query` from the server.
///
/// Calls `GET /api/stops?q=<query>`.
///
/// # Errors
/// Returns a `String` describing the error if the HTTP request or JSON
/// deserialisation fails.
pub async fn fetch_stops(query: &str) -> Result<Vec<ApiStop>, String> {
    let mut url = base_url()?.join("/api/stops").map_err(|e| e.to_string())?;
    url.query_pairs_mut().append_pair("q", query);
    reqwest::get(url.as_str())
        .await
        .map_err(|e| fmt_err(&e))?
        .json::<Vec<ApiStop>>()
        .await
        .map_err(|e| fmt_err(&e))
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
    let mut url = base_url()?.join("/api/schedule").map_err(|e| e.to_string())?;
    url.query_pairs_mut().append_pair("stop_ids", &stop_ids.join(","));
    reqwest::get(url.as_str())
        .await
        .map_err(|e| fmt_err(&e))?
        .json::<Vec<ApiDeparture>>()
        .await
        .map_err(|e| fmt_err(&e))
}
