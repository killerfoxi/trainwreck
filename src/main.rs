#![deny(clippy::pedantic)]

mod gtfs;
mod realtime;

use std::path::PathBuf;

use clap::Parser;
use color_eyre::eyre::{self, WrapErr};

#[derive(Parser)]
#[command(about = "Display GTFS transit schedules with optional real-time updates")]
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
        println!("\nShowing schedule for first match: {}", matches[0].stop_name);
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

    if let Some(feed) = realtime {
        print_departures(&schedule, &feed);
        return Ok(());
    }

    let summaries = schedule.route_summaries();
    if summaries.is_empty() {
        println!("  No trips found.");
        return Ok(());
    }
    for summary in &summaries {
        println!("  {summary}");
    }

    Ok(())
}

fn print_departures(schedule: &gtfs::StopSchedule, feed: &realtime::RealtimeFeed) {
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

        let status_str = feed
            .status_for(&trip.trip_id, &schedule.stop.stop_id)
            .map(|s| s.to_string())
            .unwrap_or_default();

        println!(
            "{} | {:<6} → {:<30} {}",
            st.departure_time, route_name, headsign, status_str
        );
    }
}
