#![allow(clippy::module_name_repetitions)]

mod archive;
mod error;
mod model;
mod query;

pub use archive::GtfsArchive;
pub use query::StopSchedule;
