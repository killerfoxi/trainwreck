//! HTTP server for `trainwreck web`: REST API + optional static web app.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use jiff::Zoned;
use serde::{Deserialize, Serialize};
use trainwreck_core::{gtfs::GtfsArchive, realtime, DepartureStatus};

// ── state ────────────────────────────────────────────────────────────────────

/// Shared server state loaded from the GTFS archive at startup.
pub struct ServerState {
    pub gtfs: GtfsArchive,
    pub api_key: Option<String>,
    pub timezone: Option<String>,
}

type SharedState = Arc<ServerState>;

// ── response types ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct StopItem {
    stop_id: String,
    stop_name: String,
}

#[derive(Serialize)]
struct DepartureItem {
    trip_id: String,
    stop_id: String,
    /// Seconds since midnight of the service day.
    departure_secs: u32,
    route_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    route_type: Option<u16>,
    destination: String,
    /// `None` = no real-time data; positive = late, negative = early.
    #[serde(skip_serializing_if = "Option::is_none")]
    delay_secs: Option<i32>,
    canceled: bool,
    skipped: bool,
}

// ── query param types ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct StopsParams {
    q: Option<String>,
}

#[derive(Deserialize)]
struct ScheduleParams {
    stop_ids: String,
}

// ── helpers ───────────────────────────────────────────────────────────────────

#[must_use]
fn api_err(e: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

// ── handlers ──────────────────────────────────────────────────────────────────

/// `GET /api/stops?q=<query>`
///
/// Returns stops whose names contain `q` (case-insensitive). When `q` is
/// absent or empty, all stops are returned.
async fn stops_handler(
    State(state): State<SharedState>,
    Query(params): Query<StopsParams>,
) -> Result<Json<Vec<StopItem>>, (StatusCode, String)> {
    let stops = match params.q.as_deref().filter(|q| !q.is_empty()) {
        Some(q) => state.gtfs.find_stops(q),
        None => state.gtfs.stops(),
    }
    .map_err(api_err)?;

    Ok(Json(
        stops
            .into_iter()
            .map(|s| StopItem { stop_id: s.stop_id, stop_name: s.stop_name })
            .collect(),
    ))
}

/// `GET /api/schedule?stop_ids=<id1,id2,...>`
///
/// Returns departures for the given stop IDs using today's active service
/// calendar. Real-time delay data is merged in when an API key is configured.
async fn schedule_handler(
    State(state): State<SharedState>,
    Query(params): Query<ScheduleParams>,
) -> Result<Json<Vec<DepartureItem>>, (StatusCode, String)> {
    let stop_ids: Vec<&str> = params
        .stop_ids
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    if stop_ids.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "stop_ids must not be empty".into()));
    }

    // Resolve timezone: explicit flag → agency.txt → system local.
    let now = Zoned::now();
    let tz_name = match &state.timezone {
        Some(tz) => Some(tz.clone()),
        None => state.gtfs.agency_timezone().map_err(api_err)?,
    };
    let now = match tz_name.as_deref() {
        Some(tz) => now.in_tz(tz).map_err(api_err)?,
        None => now,
    };

    let active_services = state.gtfs.active_service_ids(now.date()).map_err(api_err)?;
    let schedule = state
        .gtfs
        .schedule_for_stops(&stop_ids, active_services.as_ref())
        .map_err(api_err)?;

    // Fetch real-time data if an API key is configured; failures are non-fatal.
    let feed = if let Some(key) = &state.api_key {
        realtime::fetch_trip_updates(key)
            .await
            .inspect_err(|e| eprintln!("Warning: real-time unavailable: {e}"))
            .ok()
    } else {
        None
    };

    let items = schedule
        .departures()
        .into_iter()
        .map(|(st, trip, route)| {
            let route_name = route
                .and_then(|r| r.route_short_name.as_deref().or(r.route_long_name.as_deref()))
                .unwrap_or(trip.route_id.as_str())
                .to_string();
            let route_type = route.and_then(|r| r.route_type);
            let destination = trip.trip_headsign.clone().unwrap_or_else(|| "?".into());

            let (delay_secs, canceled, skipped) = feed
                .as_ref()
                .and_then(|f| f.status_for(&trip.trip_id, &st.stop_id))
                .map_or((None, false, false), |s| match s {
                    DepartureStatus::Canceled => (None, true, false),
                    DepartureStatus::Skipped => (None, false, true),
                    DepartureStatus::OnTime { delay_secs } => (Some(delay_secs), false, false),
                });

            DepartureItem {
                trip_id: trip.trip_id.clone(),
                stop_id: st.stop_id.clone(),
                departure_secs: st.departure_time.as_secs(),
                route_name,
                route_type,
                destination,
                delay_secs,
                canceled,
                skipped,
            }
        })
        .collect();

    Ok(Json(items))
}

async fn fallback_index() -> impl IntoResponse {
    Html(concat!(
        "<!DOCTYPE html><html lang=\"en\"><head>",
        "<meta charset=\"utf-8\">",
        "<title>trainwreck</title>",
        "<style>body{font-family:monospace;max-width:600px;margin:2rem auto}</style>",
        "</head><body>",
        "<h1>trainwreck API server</h1>",
        "<p>The web UI is not built yet. Run <code>trunk build</code> inside ",
        "<code>crates/web/</code>, then restart with ",
        "<code>--web-dir crates/web/dist</code>.</p>",
        "<h2>REST API</h2>",
        "<ul>",
        "<li><code>GET /api/stops?q=query</code> — search stops by name</li>",
        "<li><code>GET /api/schedule?stop_ids=id1,id2</code> — departures with real-time</li>",
        "</ul>",
        "</body></html>",
    ))
}

// ── server entry point ────────────────────────────────────────────────────────

/// Start the HTTP server and block until it exits.
///
/// Mounts `/api/stops` and `/api/schedule` as JSON endpoints.  When `web_dir`
/// is provided, static files from that directory are served at `/`, with
/// `index.html` as the SPA fallback for unmatched paths.
///
/// # Errors
/// Returns an error if the TCP listener cannot bind to `addr`, or if the
/// server encounters a fatal error while running.
pub async fn run_server(
    state: ServerState,
    addr: SocketAddr,
    web_dir: Option<PathBuf>,
) -> color_eyre::eyre::Result<()> {
    let state = Arc::new(state);

    let app = Router::new()
        .route("/api/stops", get(stops_handler))
        .route("/api/schedule", get(schedule_handler))
        .with_state(Arc::clone(&state));

    let app = match web_dir {
        Some(dir) => {
            use tower_http::services::{ServeDir, ServeFile};
            let index = ServeFile::new(dir.join("index.html"));
            app.fallback_service(ServeDir::new(&dir).not_found_service(index))
        }
        None => app.route("/", get(fallback_index)),
    };

    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
