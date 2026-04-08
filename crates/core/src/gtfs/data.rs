//! Eager in-memory GTFS store for the web interface.
//!
//! Unlike [`GtfsArchive`] (which streams from a ZIP file on disk one query at
//! a time), `GtfsData` reads a ZIP byte-slice in one shot and keeps everything
//! in memory — appropriate for the web version where there is no filesystem
//! and no memory constraint.

use std::collections::{HashMap, HashSet};
use std::io::{BufReader, Cursor, Read};

use jiff::civil::Date;
use zip::ZipArchive;

use super::error::GtfsError;
use super::model::{Agency, Calendar, CalendarDate, Route, Stop, StopTime, Trip};
use super::query::StopSchedule;

#[derive(Clone)]
pub struct GtfsData {
    pub stops:       Vec<Stop>,
    pub stop_times:  Vec<StopTime>,
    pub trips:       HashMap<String, Trip>,
    pub routes:      HashMap<String, Route>,
    calendars:       Vec<Calendar>,
    calendar_dates:  Vec<CalendarDate>,
    agency_timezone: Option<String>,
}

impl GtfsData {
    /// Parse a GTFS ZIP from a byte slice (e.g. a browser File upload).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, GtfsError> {
        let cursor = Cursor::new(bytes);
        let mut zip = ZipArchive::new(cursor).map_err(|e| GtfsError::Open {
            path: std::path::PathBuf::from("<in-memory>"),
            source: e,
        })?;

        let stops      = read_csv(&mut zip, "stops.txt")?;
        let stop_times = read_csv(&mut zip, "stop_times.txt")?;

        let trips: HashMap<String, Trip> = read_csv::<Trip>(&mut zip, "trips.txt")?
            .into_iter()
            .map(|t| (t.trip_id.clone(), t))
            .collect();

        let routes: HashMap<String, Route> = read_csv::<Route>(&mut zip, "routes.txt")?
            .into_iter()
            .map(|r| (r.route_id.clone(), r))
            .collect();

        let calendars = read_optional_csv::<Calendar>(&mut zip, "calendar.txt")?
            .unwrap_or_default();
        let calendar_dates =
            read_optional_csv::<CalendarDate>(&mut zip, "calendar_dates.txt")?
                .unwrap_or_default();

        let agency_timezone = read_optional_csv::<Agency>(&mut zip, "agency.txt")?
            .and_then(|v| v.into_iter().next())
            .map(|a| a.agency_timezone);

        Ok(Self { stops, stop_times, trips, routes, calendars, calendar_dates, agency_timezone })
    }

    /// Case-insensitive substring search on stop names. Returns up to 30 matches.
    pub fn find_stops(&self, query: &str) -> Vec<&Stop> {
        let q = query.to_lowercase();
        self.stops
            .iter()
            .filter(|s| s.stop_name.to_lowercase().contains(&q))
            .take(30)
            .collect()
    }

    /// Service IDs active on `date`, or `None` if no calendar data is present.
    pub fn active_service_ids(&self, date: Date) -> Option<HashSet<String>> {
        if self.calendars.is_empty() && self.calendar_dates.is_empty() {
            return None;
        }
        let date_str = format!("{:04}{:02}{:02}", date.year(), date.month(), date.day());
        let mut active = HashSet::new();
        for cal in &self.calendars {
            if cal.start_date.as_str() <= date_str.as_str()
                && date_str.as_str() <= cal.end_date.as_str()
                && cal.runs_on(date.weekday())
            {
                active.insert(cal.service_id.clone());
            }
        }
        for cd in self.calendar_dates.iter().filter(|cd| cd.date == date_str) {
            match cd.exception_type {
                1 => { active.insert(cd.service_id.clone()); }
                2 => { active.remove(&cd.service_id); }
                _ => {}
            }
        }
        Some(active)
    }

    /// Build a `StopSchedule` from in-memory data (no I/O).
    pub fn schedule_for_stops(
        &self,
        stop_ids: &[&str],
        active_services: Option<&HashSet<String>>,
    ) -> StopSchedule {
        let stop_id_set: HashSet<&str> = stop_ids.iter().copied().collect();

        let stop_times: Vec<StopTime> = self.stop_times.iter()
            .filter(|st| stop_id_set.contains(st.stop_id.as_str()))
            .cloned()
            .collect();

        let trip_ids: HashSet<&str> = stop_times.iter()
            .map(|st| st.trip_id.as_str())
            .collect();

        let trips: HashMap<String, Trip> = self.trips.iter()
            .filter(|(id, trip)| {
                trip_ids.contains(id.as_str())
                    && active_services
                        .map(|s| s.contains(&trip.service_id))
                        .unwrap_or(true)
            })
            .map(|(id, t)| (id.clone(), t.clone()))
            .collect();

        let route_ids: HashSet<&str> = trips.values()
            .map(|t| t.route_id.as_str())
            .collect();

        let routes: HashMap<String, Route> = self.routes.iter()
            .filter(|(id, _)| route_ids.contains(id.as_str()))
            .map(|(id, r)| (id.clone(), r.clone()))
            .collect();

        StopSchedule { stop_times, trips, routes }
    }

    pub fn agency_timezone(&self) -> Option<&str> {
        self.agency_timezone.as_deref()
    }

    pub fn stop_count(&self) -> usize {
        self.stops.len()
    }
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn read_csv<T: serde::de::DeserializeOwned>(
    zip: &mut ZipArchive<Cursor<&[u8]>>,
    name: &'static str,
) -> Result<Vec<T>, GtfsError> {
    read_optional_csv(zip, name)?
        .ok_or(GtfsError::MissingFile { name })
}

fn read_optional_csv<T: serde::de::DeserializeOwned>(
    zip: &mut ZipArchive<Cursor<&[u8]>>,
    name: &'static str,
) -> Result<Option<Vec<T>>, GtfsError> {
    let entry = match zip.by_name(name) {
        Ok(e) => e,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(GtfsError::ReadEntry { name, source: e }),
    };
    let mut buf = Vec::new();
    BufReader::new(entry)
        .read_to_end(&mut buf)
        .map_err(|e| GtfsError::Csv { file: name, source: e.into() })?;
    csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(buf.as_slice())
        .deserialize::<T>()
        .collect::<Result<Vec<T>, _>>()
        .map(Some)
        .map_err(|e| GtfsError::Csv { file: name, source: e })
}
