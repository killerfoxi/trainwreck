//! Browser-compatible time helpers using the JS Date API.
//! We don't bundle a timezone database in WASM; the browser's local
//! time is used directly via js_sys::Date.

use js_sys::Date;

/// Seconds since midnight in the browser's local timezone.
pub fn now_secs_since_midnight() -> u32 {
    let d = Date::new_0();
    d.get_hours() as u32 * 3600
        + d.get_minutes() as u32 * 60
        + d.get_seconds() as u32
}

/// (year, month 1-12, day 1-31) in the browser's local timezone.
pub fn today_ymd() -> (i32, u8, u8) {
    let d = Date::new_0();
    (d.get_full_year() as i32, (d.get_month() + 1) as u8, d.get_date() as u8)
}

/// Format a GtfsTime raw value (seconds since midnight) as "HH:MM".
/// Values ≥ 86400 (post-midnight service) are wrapped to wall-clock time.
pub fn format_hhmm(secs: u32) -> String {
    let h = (secs / 3600) % 24;
    let m = (secs % 3600) / 60;
    format!("{h:02}:{m:02}")
}

/// Human-readable relative departure time.
/// - Negative (departed 1+ min ago): "2m ago"
/// - Within ±60s:                    "now"
/// - Future:                         "in 5m" / "in 1h 30m"
pub fn relative_time(departure_secs: u32, now_secs: u32) -> String {
    // Handle post-midnight departures: if departure > 12h ahead of now,
    // assume it already passed (is from previous service day).
    let dep = departure_secs as i64;
    let now = now_secs as i64;
    let diff = dep - now;

    match diff {
        d if d < -60   => format!("{}m ago", (-d) / 60),
        d if d <= 60   => "now".to_string(),
        d => {
            let mins = (d + 59) / 60; // ceiling for upcoming
            if mins >= 60 {
                format!("in {}h {}m", mins / 60, mins % 60)
            } else {
                format!("in {mins}m")
            }
        }
    }
}

/// Return a jiff::civil::Date for today using the browser's local date.
pub fn today_date() -> jiff::civil::Date {
    let (y, m, d) = today_ymd();
    jiff::civil::Date::new(y as i16, m as i8, d as i8)
        .expect("browser returned invalid date")
}
