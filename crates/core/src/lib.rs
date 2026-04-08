pub mod gtfs;
pub mod realtime;

pub use gtfs::{GtfsArchive, GtfsTime, StopSchedule};
pub use realtime::{fetch_trip_updates, DepartureStatus, RealtimeFeed};

#[cfg(feature = "web")]
pub use gtfs::GtfsData;
