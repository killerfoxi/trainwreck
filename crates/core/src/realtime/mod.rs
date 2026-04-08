pub mod client;
pub mod error;
pub mod model;
pub(crate) mod proto;

pub use client::fetch_trip_updates;
pub use model::{DepartureStatus, RealtimeFeed};
