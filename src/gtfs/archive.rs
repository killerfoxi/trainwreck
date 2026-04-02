use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use jiff::civil::Date;
use zip::ZipArchive;

use super::error::GtfsError;
use super::model::{Agency, Calendar, CalendarDate, Route, Stop, StopTime, Trip};
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
    pub fn agency_timezone(&self) -> Result<Option<String>, GtfsError> {
        let mut archive = self.archive()?;
        let entry = match archive.by_name("agency.txt") {
            Ok(e) => e,
            Err(zip::result::ZipError::FileNotFound) => return Ok(None),
            Err(e) => return Err(GtfsError::ReadEntry { name: "agency.txt", source: e }),
        };
        let mut buf = Vec::new();
        BufReader::new(entry)
            .read_to_end(&mut buf)
            .map_err(|e| GtfsError::Csv { file: "agency.txt", source: e.into() })?;
        Ok(csv::ReaderBuilder::new()
            .flexible(true)
            .from_reader(buf.as_slice())
            .deserialize::<Agency>()
            .next()
            .transpose()
            .map_err(|e| GtfsError::Csv { file: "agency.txt", source: e })?
            .map(|a| a.agency_timezone))
    }

    /// List all stops in the archive.
    ///
    /// Streams `stops.txt` without loading the entire archive.
    pub fn stops(&self) -> Result<Vec<Stop>, GtfsError> {
        self.read_csv("stops.txt")
    }

    /// Search stops by name (case-insensitive substring match).
    pub fn find_stops(&self, query: &str) -> Result<Vec<Stop>, GtfsError> {
        let query_lower = query.to_lowercase();
        self.read_csv_filtered("stops.txt", |s: &Stop| {
            s.stop_name.to_lowercase().contains(&query_lower)
        })
    }

    /// Return the set of `service_id`s active on `date`, or `None` if the archive
    /// contains no calendar data (in which case callers should treat all services as active).
    ///
    /// Applies both `calendar.txt` (regular schedule) and `calendar_dates.txt` (exceptions).
    /// See <https://gtfs.org/documentation/schedule/reference/#calendartxt>.
    pub fn active_service_ids(&self, date: Date) -> Result<Option<HashSet<String>>, GtfsError> {
        let date_str = format!(
            "{:04}{:02}{:02}",
            date.year(),
            i8::from(date.month()),
            date.day(),
        );
        let mut has_data = false;
        let mut active: HashSet<String> = HashSet::new();

        if let Some(calendars) = self.read_optional_csv::<Calendar>("calendar.txt")? {
            has_data = true;
            for cal in calendars {
                if cal.start_date.as_str() <= date_str.as_str()
                    && date_str.as_str() <= cal.end_date.as_str()
                    && cal.runs_on(date.weekday())
                {
                    active.insert(cal.service_id);
                }
            }
        }

        if let Some(exceptions) =
            self.read_optional_csv_filtered::<CalendarDate>("calendar_dates.txt", |cd| {
                cd.date == date_str
            })?
        {
            has_data = true;
            for cd in exceptions {
                match cd.exception_type {
                    1 => { active.insert(cd.service_id); }
                    2 => { active.remove(&cd.service_id); }
                    _ => {}
                }
            }
        }

        Ok(has_data.then_some(active))
    }

    /// Build a schedule for the given stops, optionally filtered to active service IDs.
    ///
    /// Strategy (memory-efficient):
    /// 1. Stream `stop_times.txt`, collecting rows for any of the given stop IDs.
    /// 2. Stream `trips.txt`, keeping only trips referenced above that are also in `active_services`.
    /// 3. Stream `routes.txt`, keeping only routes referenced by those trips.
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
                && active_services.map_or(true, |s| s.contains(t.service_id.as_str()))
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
