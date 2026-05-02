pub mod gtfs;
pub mod realtime;

pub use gtfs::{GtfsArchive, GtfsTime, StopSchedule};
pub use realtime::{DepartureStatus, RealtimeFeed, fetch_trip_updates};

#[cfg(feature = "web")]
pub use gtfs::GtfsData;
