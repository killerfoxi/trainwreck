//! Browser-compatible time helpers using the JS Date API.
//! We don't bundle a timezone database in WASM; the browser's local
//! time is used directly via `js_sys::Date`.

use js_sys::Date;

/// Seconds since midnight in the browser's local timezone.
#[must_use]
pub fn now_secs_since_midnight() -> u32 {
    let d = Date::new_0();
    // JS Date.getHours/getMinutes/getSeconds return u32-range values.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let (h, min, s) = (
        d.get_hours() as u32,
        d.get_minutes() as u32,
        d.get_seconds() as u32,
    );
    h * 3600 + min * 60 + s
}

/// Returns `(year, month 1-12, day 1-31)` in the browser's local timezone.
#[must_use]
pub fn today_ymd() -> (i32, u8, u8) {
    let d = Date::new_0();
    // JS Date returns u32-range integers; the casts are safe for valid dates.
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let (month, day) = ((d.get_month() + 1) as u8, d.get_date() as u8);
    let year = d.get_full_year().cast_signed();
    (year, month, day)
}

/// Format a `GtfsTime` raw value (seconds since midnight) as `"HH:MM"`.
/// Values ≥ 86400 (post-midnight service) are wrapped to wall-clock time.
#[must_use]
pub fn format_hhmm(secs: u32) -> String {
    let h = (secs / 3600) % 24;
    let m = (secs % 3600) / 60;
    format!("{h:02}:{m:02}")
}

/// Human-readable relative departure time.
/// - Negative (departed 1+ min ago): `"2m ago"`
/// - Within ±60s:                    `"now"`
/// - Future:                         `"in 5m"` / `"in 1h 30m"`
#[must_use]
pub fn relative_time(departure_secs: u32, now_secs: u32) -> String {
    // Handle post-midnight departures: if departure > 12h ahead of now,
    // assume it already passed (is from previous service day).
    let dep = i64::from(departure_secs);
    let now = i64::from(now_secs);
    let diff = dep - now;

    match diff {
        ..=-61 => format!("{}m ago", (-diff) / 60),
        -60..=60 => "now".to_string(),
        _ => {
            let mins = (diff + 59) / 60; // ceiling for upcoming
            if mins >= 60 {
                format!("in {}h {}m", mins / 60, mins % 60)
            } else {
                format!("in {mins}m")
            }
        }
    }
}

/// Return a [`jiff::civil::Date`] for today using the browser's local date.
///
/// # Panics
/// Panics if the browser returns an invalid date (year/month/day out of range),
/// which cannot happen in a conforming browser environment.
#[must_use]
pub fn today_date() -> jiff::civil::Date {
    let (y, m, d) = today_ymd();
    // JS Date month is 1-12 and day is 1-31, both fit in i8 for valid dates.
    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let (month, day) = (m as i8, d as i8);
    #[allow(clippy::cast_possible_truncation)]
    let year = y as i16;
    jiff::civil::Date::new(year, month, day).expect("browser returned invalid date")
}
