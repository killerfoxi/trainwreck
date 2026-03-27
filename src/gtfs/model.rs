// Field names are dictated by the GTFS CSV specification.
#![allow(clippy::struct_field_names, dead_code)]

use serde::Deserialize;

/// A transit stop/station.
#[derive(Debug, Clone, Deserialize)]
pub struct Stop {
    pub stop_id: String,
    pub stop_name: String,
    #[serde(default)]
    pub stop_lat: Option<f64>,
    #[serde(default)]
    pub stop_lon: Option<f64>,
}

/// A scheduled stop-time linking a trip to a stop.
#[derive(Debug, Clone, Deserialize)]
pub struct StopTime {
    pub trip_id: String,
    pub arrival_time: String,
    pub departure_time: String,
    pub stop_id: String,
    pub stop_sequence: u32,
}

/// A trip along a route.
#[derive(Debug, Clone, Deserialize)]
pub struct Trip {
    pub route_id: String,
    pub service_id: String,
    pub trip_id: String,
    #[serde(default)]
    pub trip_headsign: Option<String>,
    #[serde(default)]
    pub direction_id: Option<u8>,
}

/// A transit route.
#[derive(Debug, Clone, Deserialize)]
pub struct Route {
    pub route_id: String,
    #[serde(default)]
    pub route_short_name: Option<String>,
    #[serde(default)]
    pub route_long_name: Option<String>,
    #[serde(default)]
    pub route_type: Option<RouteType>,
}

/// GTFS route types (subset covering rail).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(from = "u16")]
pub enum RouteType {
    Tram,
    Subway,
    Rail,
    Bus,
    Ferry,
    CableTram,
    Gondola,
    Funicular,
    Trolleybus,
    Monorail,
    Other(u16),
}

impl From<u16> for RouteType {
    fn from(value: u16) -> Self {
        match value {
            0 => Self::Tram,
            1 => Self::Subway,
            2 => Self::Rail,
            3 => Self::Bus,
            4 => Self::Ferry,
            5 => Self::CableTram,
            6 => Self::Gondola,
            7 => Self::Funicular,
            11 => Self::Trolleybus,
            12 => Self::Monorail,
            n => Self::Other(n),
        }
    }
}

impl std::fmt::Display for RouteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tram => write!(f, "Tram"),
            Self::Subway => write!(f, "Subway"),
            Self::Rail => write!(f, "Rail"),
            Self::Bus => write!(f, "Bus"),
            Self::Ferry => write!(f, "Ferry"),
            Self::CableTram => write!(f, "Cable Tram"),
            Self::Gondola => write!(f, "Gondola"),
            Self::Funicular => write!(f, "Funicular"),
            Self::Trolleybus => write!(f, "Trolleybus"),
            Self::Monorail => write!(f, "Monorail"),
            Self::Other(n) => write!(f, "Other({n})"),
        }
    }
}
