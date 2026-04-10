use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use jiff::civil::Date;
use zip::ZipArchive;

use super::error::GtfsError;
use super::model::{Agency, Calendar, CalendarDate, ExceptionType, Route, Stop, StopTime, Trip};
use super::query::StopSchedule;

/// Handle to a GTFS ZIP archive.
///
/// Reads are streamed entry-by-entry to keep memory usage low.
/// Only the data needed for a given query is loaded.
pub struct GtfsArchive {
    path: std::path::PathBuf,
}

impl GtfsArchive {
    /// Open a GTFS archive, validating that required files exist.
    ///
    /// # Errors
    /// Returns [`GtfsError::Open`] if the path cannot be opened or is not a valid ZIP,
    /// or [`GtfsError::MissingFile`] if any required file is absent.
    pub fn open(path: &Path) -> Result<Self, GtfsError> {
        let file = File::open(path).map_err(|e| GtfsError::Open {
            path: path.to_path_buf(),
            source: e.into(),
        })?;
        let archive = ZipArchive::new(BufReader::new(file)).map_err(|e| GtfsError::Open {
            path: path.to_path_buf(),
            source: e,
        })?;

        // Required files per the GTFS Schedule spec:
        // https://gtfs.org/documentation/schedule/reference/#dataset-files
        for required in ["stops.txt", "stop_times.txt", "trips.txt", "routes.txt"] {
            if archive.index_for_name(required).is_none() {
                return Err(GtfsError::MissingFile { name: required });
            }
        }

        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    /// Read the IANA timezone name from `agency.txt`, returning `None` if absent.
    ///
    /// Per the GTFS spec, `agency_timezone` is required whenever `agency.txt` is present.
    /// See <https://gtfs.org/documentation/schedule/reference/#agency_timezone>.
    ///
    /// # Errors
    /// Returns [`GtfsError`] if the ZIP entry cannot be read or the CSV is malformed.
    pub fn agency_timezone(&self) -> Result<Option<String>, GtfsError> {
        Ok(self
            .read_optional_csv::<Agency>("agency.txt")?
            .and_then(|agencies| agencies.into_iter().next())
            .map(|a| a.agency_timezone))
    }

    /// List all stops in the archive.
    ///
    /// Streams `stops.txt` without loading the entire archive.
    ///
    /// # Errors
    /// Returns [`GtfsError`] if the ZIP entry cannot be read or the CSV is malformed.
    pub fn stops(&self) -> Result<Vec<Stop>, GtfsError> {
        self.read_csv("stops.txt")
    }

    /// Search stops by name (case-insensitive substring match).
    ///
    /// # Errors
    /// Returns [`GtfsError`] if the ZIP entry cannot be read or the CSV is malformed.
    pub fn find_stops(&self, query: &str) -> Result<Vec<Stop>, GtfsError> {
        let query_lower = query.to_lowercase();
        self.read_csv_filtered("stops.txt", |s: &Stop| {
            s.stop_name.to_lowercase().contains(&query_lower)
        })
    }

    /// Fetch the [`Stop`] records for a given set of stop IDs.
    ///
    /// # Errors
    /// Returns [`GtfsError`] if `stops.txt` cannot be read or parsed.
    pub fn stops_by_ids(
        &self,
        ids: &HashSet<&str>,
    ) -> Result<std::collections::HashMap<String, Stop>, GtfsError> {
        let stops: Vec<Stop> =
            self.read_csv_filtered("stops.txt", |s: &Stop| ids.contains(s.stop_id.as_str()))?;
        Ok(stops.into_iter().map(|s| (s.stop_id.clone(), s)).collect())
    }

    /// Expand a set of stop IDs to include their full station family.
    ///
    /// GTFS feeds often split a station into a parent (location_type=1) and
    /// child platforms (location_type=0). Stop-times are attached to platforms,
    /// not the parent. Given a set of selected stop IDs this method:
    ///
    /// 1. Locates the parent station of each selected stop (if any).
    /// 2. Adds all siblings — other children of those same parents.
    /// 3. Adds direct children of any selected stop that is itself a parent.
    ///
    /// The result is the union of all related stops, so querying the schedule
    /// with it returns departures regardless of which stop in a station group
    /// the user selected.
    ///
    /// # Errors
    /// Returns [`GtfsError`] if `stops.txt` cannot be read or parsed.
    pub fn expand_stop_ids(&self, stop_ids: &HashSet<&str>) -> Result<HashSet<String>, GtfsError> {
        let all_stops: Vec<Stop> = self.read_csv("stops.txt")?;
        Ok(expand_family(stop_ids, &all_stops))
    }

    /// Return the set of `service_id`s active on `date`, or `None` if the archive
    /// contains no calendar data (in which case callers should treat all services as active).
    ///
    /// Applies both `calendar.txt` (regular schedule) and `calendar_dates.txt` (exceptions).
    /// See <https://gtfs.org/documentation/schedule/reference/#calendartxt>.
    ///
    /// # Errors
    /// Returns [`GtfsError`] if any calendar CSV cannot be read or is malformed.
    pub fn active_service_ids(&self, date: Date) -> Result<Option<HashSet<String>>, GtfsError> {
        let date_str = format!(
            "{:04}{:02}{:02}",
            date.year(),
            date.month(),
            date.day(),
        );
        let calendars = self.read_optional_csv::<Calendar>("calendar.txt")?;
        let exceptions =
            self.read_optional_csv_filtered::<CalendarDate>("calendar_dates.txt", |cd| {
                cd.date == date_str
            })?;

        if calendars.is_none() && exceptions.is_none() {
            return Ok(None);
        }

        let mut active = HashSet::new();

        if let Some(calendars) = calendars {
            active.extend(
                calendars
                    .into_iter()
                    .filter(|cal| {
                        cal.start_date.as_str() <= date_str.as_str()
                            && date_str.as_str() <= cal.end_date.as_str()
                            && cal.runs_on(date.weekday())
                    })
                    .map(|cal| cal.service_id),
            );
        }

        if let Some(exceptions) = exceptions {
            for cd in exceptions {
                match cd.exception_type {
                    ExceptionType::Added   => { active.insert(cd.service_id); }
                    ExceptionType::Removed => { active.remove(&cd.service_id); }
                }
            }
        }

        Ok(Some(active))
    }

    /// Build a schedule for the given stops, optionally filtered to active service IDs.
    ///
    /// Strategy (memory-efficient):
    /// 1. Stream `stop_times.txt`, collecting rows for any of the given stop IDs.
    /// 2. Stream `trips.txt`, keeping only trips referenced above that are also in `active_services`.
    /// 3. Stream `routes.txt`, keeping only routes referenced by those trips.
    ///
    /// # Errors
    /// Returns [`GtfsError`] if any CSV entry cannot be read or is malformed.
    pub fn schedule_for_stops(
        &self,
        stop_ids: &[&str],
        active_services: Option<&HashSet<String>>,
    ) -> Result<StopSchedule, GtfsError> {
        let stop_times: Vec<StopTime> = self.read_csv_filtered("stop_times.txt", |st: &StopTime| {
            stop_ids.contains(&st.stop_id.as_str())
        })?;

        let trip_ids: HashSet<&str> = stop_times.iter().map(|st| st.trip_id.as_str()).collect();

        let trips: Vec<Trip> = self.read_csv_filtered("trips.txt", |t: &Trip| {
            trip_ids.contains(t.trip_id.as_str())
                && active_services.is_none_or(|s| s.contains(t.service_id.as_str()))
        })?;

        let route_ids: HashSet<&str> = trips.iter().map(|t| t.route_id.as_str()).collect();

        let routes: Vec<Route> = self.read_csv_filtered("routes.txt", |r: &Route| {
            route_ids.contains(r.route_id.as_str())
        })?;

        Ok(StopSchedule {
            stop_times,
            trips: trips.into_iter().map(|t| (t.trip_id.clone(), t)).collect(),
            routes: routes.into_iter().map(|r| (r.route_id.clone(), r)).collect(),
        })
    }

    fn read_optional_csv<T: serde::de::DeserializeOwned>(
        &self,
        name: &'static str,
    ) -> Result<Option<Vec<T>>, GtfsError> {
        self.read_optional_csv_filtered(name, |_| true)
    }

    fn read_optional_csv_filtered<T: serde::de::DeserializeOwned>(
        &self,
        name: &'static str,
        predicate: impl Fn(&T) -> bool,
    ) -> Result<Option<Vec<T>>, GtfsError> {
        let mut archive = self.archive()?;
        let entry = match archive.by_name(name) {
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
            .filter_map(|r| match r {
                Ok(item) if predicate(&item) => Some(Ok(item)),
                Ok(_) => None,
                Err(e) => Some(Err(GtfsError::Csv { file: name, source: e })),
            })
            .collect::<Result<Vec<T>, _>>()
            .map(Some)
    }

    fn archive(&self) -> Result<ZipArchive<BufReader<File>>, GtfsError> {
        let file = File::open(&self.path).map_err(|e| GtfsError::Open {
            path: self.path.clone(),
            source: e.into(),
        })?;
        ZipArchive::new(BufReader::new(file)).map_err(|e| GtfsError::Open {
            path: self.path.clone(),
            source: e,
        })
    }

    /// Read and deserialize an entire CSV entry from the ZIP.
    fn read_csv<T: serde::de::DeserializeOwned>(&self, name: &'static str) -> Result<Vec<T>, GtfsError> {
        self.read_csv_filtered(name, |_| true)
    }

    /// Stream a CSV entry, keeping only rows that pass `predicate`.
    ///
    /// Reads the entry into a buffer (zip entries aren't seekable), then
    /// filters via iterator combinators — no intermediate mutable accumulator.
    fn read_csv_filtered<T: serde::de::DeserializeOwned>(
        &self,
        name: &'static str,
        predicate: impl Fn(&T) -> bool,
    ) -> Result<Vec<T>, GtfsError> {
        let mut archive = self.archive()?;
        let entry = archive
            .by_name(name)
            .map_err(|e| GtfsError::ReadEntry { name, source: e })?;

        let mut buf = Vec::new();
        BufReader::new(entry)
            .read_to_end(&mut buf)
            .map_err(|e| GtfsError::Csv {
                file: name,
                source: e.into(),
            })?;

        csv::ReaderBuilder::new()
            .flexible(true)
            .from_reader(buf.as_slice())
            .deserialize::<T>()
            .filter_map(|r| match r {
                Ok(item) if predicate(&item) => Some(Ok(item)),
                Ok(_) => None,
                Err(e) => Some(Err(GtfsError::Csv { file: name, source: e })),
            })
            .collect()
    }
}

// ── shared stop-family expansion ──────────────────────────────────────────────

/// Given a set of stop IDs and the full stop list, return the union of:
/// - the selected stops themselves
/// - their parent stations (if any)
/// - all children of those parents (siblings of the selected stops)
/// - all direct children of the selected stops (if they are themselves parents)
///
/// This ensures that selecting any stop in a station hierarchy (parent station,
/// platform, or entrance) returns the full set of stops with actual stop-times.
pub(super) fn expand_family(stop_ids: &HashSet<&str>, all_stops: &[Stop]) -> HashSet<String> {
    // Collect parent_station IDs of the selected stops.
    let parents: HashSet<&str> = all_stops
        .iter()
        .filter(|s| stop_ids.contains(s.stop_id.as_str()))
        .filter_map(|s| s.parent_station.as_deref())
        .filter(|p| !p.is_empty())
        .collect();

    // A stop belongs in the expanded set if:
    // - it was directly selected, OR
    // - it is a parent of a selected stop, OR
    // - its parent_station is one of the selected stops or their parents (sibling/child).
    all_stops
        .iter()
        .filter(|s| {
            let id = s.stop_id.as_str();
            let parent = s.parent_station.as_deref().unwrap_or("");
            stop_ids.contains(id)
                || parents.contains(id)
                || stop_ids.contains(parent)
                || parents.contains(parent)
        })
        .map(|s| s.stop_id.clone())
        .collect()
}
