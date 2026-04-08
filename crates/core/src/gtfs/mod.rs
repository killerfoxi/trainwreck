#![allow(clippy::module_name_repetitions)]

pub mod archive;
pub mod error;
pub mod model;
pub mod query;

#[cfg(feature = "web")]
pub mod data;

pub use archive::GtfsArchive;
pub use model::GtfsTime;
pub use query::StopSchedule;

#[cfg(feature = "web")]
pub use data::GtfsData;
