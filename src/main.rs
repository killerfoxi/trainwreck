#![deny(clippy::pedantic)]

mod gtfs;
mod realtime;

use std::path::PathBuf;

use clap::Parser;
use color_eyre::eyre::{self, WrapErr};
use jiff::{ToSpan as _, Zoned};

#[derive(Parser)]
#[command(
    version,
    about = "Display GTFS transit schedules with optional real-time updates"
)]
struct Args {
    /// Path to a GTFS zip archive
    gtfs: PathBuf,

    /// Stop name to search for (omit to list all stops)
    stop: Option<String>,

    /// API key for real-time data (or set `OTD_API_KEY` env var)
    #[arg(long, env = "OTD_API_KEY")]
    api_key: Option<String>,

    /// IANA timezone for interpreting GTFS departure times (e.g. "Europe/Zurich").
    /// Defaults to the timezone declared in agency.txt, falling back to the system timezone.
    #[arg(long)]
    timezone: Option<String>,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    let archive = gtfs::GtfsArchive::open(&args.gtfs)
        .wrap_err_with(|| format!("opening GTFS archive {}", args.gtfs.display()))?;

    match args.stop {
        Some(q) => show_schedule(&archive, &q, args.api_key.as_deref(), args.timezone.as_deref()).await?,
        None => list_stops(&archive)?,
    }

    Ok(())
}

fn list_stops(archive: &gtfs::GtfsArchive) -> eyre::Result<()> {
    let stops = archive.stops().wrap_err("reading stops")?;
    println!("Found {} stops:", stops.len());
    for stop in &stops {
        println!("  [{}] {}", stop.stop_id, stop.stop_name);
    }
    Ok(())
}

async fn show_schedule(
    archive: &gtfs::GtfsArchive,
    query: &str,
    api_key: Option<&str>,
    timezone: Option<&str>,
) -> eyre::Result<()> {
    let matches = archive.find_stops(query).wrap_err("searching stops")?;

    if matches.is_empty() {
        return Err(eyre::eyre!("no stops matching \"{query}\""));
    }

    let tz_name = match timezone {
        Some(tz) => Some(tz.to_owned()),
        None => archive.agency_timezone().wrap_err("reading agency timezone")?,
    };
    let now = Zoned::now();
    let now = match tz_name.as_deref() {
        Some(tz) => now.in_tz(tz).wrap_err_with(|| format!("unknown timezone \"{tz}\""))?,
        None => now,
    };

    let active_services = archive
        .active_service_ids(now.date())
        .wrap_err("reading calendar")?;

    let stop_ids: Vec<&str> = matches.iter().map(|s| s.stop_id.as_str()).collect();
    let schedule = archive
        .schedule_for_stops(&stop_ids, active_services.as_ref())
        .wrap_err("building schedule")?;

    if matches.len() == 1 {
        println!("\nSchedule for {} ({}):", matches[0].stop_name, matches[0].stop_id);
    } else {
        println!("\nSchedule for \"{}\" ({} stops):", query, matches.len());
        for stop in &matches {
            println!("  [{}] {}", stop.stop_id, stop.stop_name);
        }
    }

    let realtime = async {
        realtime::fetch_trip_updates(api_key?)
            .await
            .inspect_err(|e| eprintln!("Warning: real-time data unavailable: {e}"))
            .ok()
    }
    .await;

    print_departures(&schedule, realtime.as_ref(), &now);
    Ok(())
}

fn print_departures(
    schedule: &gtfs::StopSchedule,
    feed: Option<&realtime::RealtimeFeed>,
    now: &Zoned,
) {
    let departures = schedule.departures();
    if departures.is_empty() {
        println!("  No trips found.");
        return;
    }
    for (st, trip, route) in departures {
        let route_name = route
            .and_then(|r| r.route_short_name.as_deref().or(r.route_long_name.as_deref()))
            .unwrap_or(trip.route_id.as_str());
        let headsign = trip.trip_headsign.as_deref().unwrap_or("?");
        let rel = relative_time(st.departure_time, now);
        let status_str = feed
            .and_then(|f| f.status_for(&trip.trip_id, &st.stop_id))
            .map(|s| s.to_string())
            .unwrap_or_default();

        println!(
            "{} {:<8} | {:<6} → {:<30} {}",
            st.departure_time, rel, route_name, headsign, status_str
        );
    }
}

/// Returns a human-readable duration relative to `now`: "now", "in 5m", "in 1h 30m", "3m ago".
/// Uses today's service date; GTFS times ≥ 24h are treated as next-calendar-day departures.
fn relative_time(dep: gtfs::GtfsTime, now: &Zoned) -> String {
    let tz = now.time_zone().clone();
    let Ok(midnight) = now.date().at(0, 0, 0, 0).to_zoned(tz) else {
        return String::new();
    };
    let departure = midnight + i64::from(dep.as_secs()).seconds();
    let diff_secs = departure.timestamp().as_second() - now.timestamp().as_second();
    // Ceiling division for upcoming trains so "in 2m" is shown until the moment of departure,
    // floor for past trains since they've already been gone that many whole minutes.
    match diff_secs {
        0 => "now".to_owned(),
        1.. => format!("in {}", fmt_duration(((diff_secs + 59) / 60).cast_unsigned())),
        _ => format!("{} ago", fmt_duration((-diff_secs / 60).cast_unsigned())),
    }
}

fn fmt_duration(mins: u64) -> String {
    let (h, m) = (mins / 60, mins % 60);
    match (h, m) {
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h {m}m"),
    }
}
