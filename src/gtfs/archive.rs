use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use zip::ZipArchive;

use super::error::GtfsError;
use super::model::{Route, Stop, StopTime, Trip};
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

    /// Build a schedule for a given stop: all trips that serve it,
    /// with route information.
    ///
    /// Strategy (memory-efficient):
    /// 1. Stream `stop_times.txt`, collecting only rows matching `stop_id`.
    /// 2. Gather the resulting `trip_id` set (small).
    /// 3. Stream `trips.txt`, keeping only matching trips → collect `route_id` set.
    /// 4. Stream `routes.txt`, keeping only matching routes.
    pub fn schedule_for_stop(&self, stop_id: &str) -> Result<StopSchedule, GtfsError> {
        let stop = self
            .read_csv_filtered("stops.txt", |s: &Stop| s.stop_id == stop_id)?
            .into_iter()
            .next()
            .ok_or_else(|| GtfsError::StopNotFound(stop_id.to_string()))?;

        let stop_times: Vec<StopTime> = self.read_csv_filtered("stop_times.txt", |st: &StopTime| {
            st.stop_id == stop_id
        })?;

        let trip_ids: HashSet<&str> = stop_times.iter().map(|st| st.trip_id.as_str()).collect();

        let trips: Vec<Trip> =
            self.read_csv_filtered("trips.txt", |t: &Trip| trip_ids.contains(t.trip_id.as_str()))?;

        let route_ids: HashSet<&str> = trips.iter().map(|t| t.route_id.as_str()).collect();

        let routes: Vec<Route> = self
            .read_csv_filtered("routes.txt", |r: &Route| {
                route_ids.contains(r.route_id.as_str())
            })?;

        let route_map: HashMap<String, Route> =
            routes.into_iter().map(|r| (r.route_id.clone(), r)).collect();

        let trip_map: HashMap<String, Trip> =
            trips.into_iter().map(|t| (t.trip_id.clone(), t)).collect();

        Ok(StopSchedule {
            stop,
            stop_times,
            trips: trip_map,
            routes: route_map,
        })
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
