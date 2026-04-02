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
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    let archive = gtfs::GtfsArchive::open(&args.gtfs)
        .wrap_err_with(|| format!("opening GTFS archive {}", args.gtfs.display()))?;

    match args.stop {
        Some(q) => show_schedule(&archive, &q, args.api_key.as_deref()).await?,
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
) -> eyre::Result<()> {
    let matches = archive.find_stops(query).wrap_err("searching stops")?;

    if matches.is_empty() {
        return Err(eyre::eyre!("no stops matching \"{query}\""));
    }

    if matches.len() > 1 {
        println!("Multiple stops match \"{query}\":");
        for stop in &matches {
            println!("  [{}] {}", stop.stop_id, stop.stop_name);
        }
        println!(
            "\nShowing schedule for first match: {}",
            matches[0].stop_name
        );
    }

    let stop_id = &matches[0].stop_id;
    let schedule = archive
        .schedule_for_stop(stop_id)
        .wrap_err("building schedule")?;

    println!(
        "\nSchedule for {} ({}):",
        schedule.stop.stop_name, schedule.stop.stop_id
    );

    let realtime = if let Some(key) = api_key {
        realtime::fetch_trip_updates(key)
            .await
            .inspect_err(|e| eprintln!("Warning: real-time data unavailable: {e}"))
            .ok()
    } else {
        None
    };

    let now = Zoned::now();
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
            .and_then(|f| f.status_for(&trip.trip_id, &schedule.stop.stop_id))
            .map(|s| s.to_string())
            .unwrap_or_default();

        println!(
            "{} {:<8} | {:<6} → {:<30} {}",
            st.departure_time, rel, route_name, headsign, status_str
        );
    }
}

/// Returns "+Xm" (upcoming) or "Xm ago" (past) relative to `now`.
/// Uses today's service date; GTFS times ≥ 24h are treated as next-calendar-day departures.
fn relative_time(dep: gtfs::GtfsTime, now: &Zoned) -> String {
    let tz = now.time_zone().clone();
    let Ok(midnight) = now.date().at(0, 0, 0, 0).to_zoned(tz) else {
        return String::new();
    };
    let departure = midnight + i64::from(dep.as_secs()).seconds();
    let diff_mins =
        (departure.timestamp().as_second() - now.timestamp().as_second()) / 60;
    if diff_mins >= 0 {
        format!("+{diff_mins}m")
    } else {
        format!("{diff_mins}m ago", diff_mins = -diff_mins)
    }
}
