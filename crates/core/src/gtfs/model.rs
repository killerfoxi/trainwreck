// Field names are dictated by the GTFS CSV specification.
#![allow(clippy::struct_field_names)]

use jiff::civil::Weekday;
use serde::Deserialize;

/// A GTFS service time, which may exceed 24 hours for post-midnight trips.
///
/// The GTFS spec allows `HH:MM:SS` where HH ≥ 24 to represent times that run
/// past midnight of the service day — e.g. `25:22:00` means 01:22 AM the
/// following calendar day. See <https://gtfs.org/documentation/schedule/reference/#stop_timestxt>.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GtfsTime(u32); // seconds since midnight of the service day

impl GtfsTime {
    #[must_use]
    pub fn as_secs(self) -> u32 {
        self.0
    }
}

impl std::str::FromStr for GtfsTime {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || format!("invalid GTFS time: {s}");
        let mut parts = s.splitn(3, ':');
        let mut next_u32 = || -> Result<u32, String> {
            parts
                .next()
                .ok_or_else(invalid)?
                .parse()
                .map_err(|_| invalid())
        };
        let h = next_u32()?;
        let m = next_u32()?;
        let sec = next_u32()?;
        if m >= 60 || sec >= 60 {
            return Err(invalid());
        }
        Ok(Self(h * 3600 + m * 60 + sec))
    }
}

impl<'de> Deserialize<'de> for GtfsTime {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for GtfsTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let h = self.0 / 3600;
        let m = (self.0 % 3600) / 60;
        if h < 24 {
            return write!(f, "{h:02}:{m:02}");
        }
        // Past midnight of the service day; show wall-clock time with +1d marker.
        write!(f, "{:02}:{m:02}+1d", h - 24)
    }
}

/// A transit stop/station.
#[derive(Debug, Clone, Deserialize)]
pub struct Stop {
    pub stop_id: String,
    pub stop_name: String,
}

/// A scheduled stop-time linking a trip to a stop.
#[derive(Debug, Clone, Deserialize)]
pub struct StopTime {
    pub trip_id: String,
    pub departure_time: GtfsTime,
    pub stop_id: String,
}

/// A trip along a route.
#[derive(Debug, Clone, Deserialize)]
pub struct Trip {
    pub service_id: String,
    pub route_id: String,
    pub trip_id: String,
    #[serde(default)]
    pub trip_headsign: Option<String>,
}

/// Deserializes GTFS's `0`/`1` integer encoding of boolean flags.
fn de_bool_int<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    u8::deserialize(d).map(|v| v != 0)
}

/// A service schedule defining which days a service operates.
/// See <https://gtfs.org/documentation/schedule/reference/#calendartxt>.
// The GTFS spec has one boolean column per weekday; the lint would push us
// toward an abstraction that buys nothing over this straightforward mapping.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Deserialize)]
pub struct Calendar {
    pub service_id: String,
    #[serde(deserialize_with = "de_bool_int")]
    pub monday: bool,
    #[serde(deserialize_with = "de_bool_int")]
    pub tuesday: bool,
    #[serde(deserialize_with = "de_bool_int")]
    pub wednesday: bool,
    #[serde(deserialize_with = "de_bool_int")]
    pub thursday: bool,
    #[serde(deserialize_with = "de_bool_int")]
    pub friday: bool,
    #[serde(deserialize_with = "de_bool_int")]
    pub saturday: bool,
    #[serde(deserialize_with = "de_bool_int")]
    pub sunday: bool,
    pub start_date: String, // YYYYMMDD
    pub end_date: String,   // YYYYMMDD
}

impl Calendar {
    #[must_use]
    pub fn runs_on(&self, weekday: Weekday) -> bool {
        match weekday {
            Weekday::Monday => self.monday,
            Weekday::Tuesday => self.tuesday,
            Weekday::Wednesday => self.wednesday,
            Weekday::Thursday => self.thursday,
            Weekday::Friday => self.friday,
            Weekday::Saturday => self.saturday,
            Weekday::Sunday => self.sunday,
        }
    }
}

/// Whether a service is added or removed on a particular date.
/// See <https://gtfs.org/documentation/schedule/reference/#calendar_datestxt>.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(try_from = "u8")]
pub enum ExceptionType {
    /// Service has been added for this date (`exception_type = 1`).
    Added,
    /// Service has been removed for this date (`exception_type = 2`).
    Removed,
}

impl TryFrom<u8> for ExceptionType {
    type Error = String;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            1 => Ok(Self::Added),
            2 => Ok(Self::Removed),
            _ => Err(format!(
                "invalid exception_type {v}: expected 1 (added) or 2 (removed)"
            )),
        }
    }
}

/// A service date exception (added or removed service day).
/// See <https://gtfs.org/documentation/schedule/reference/#calendar_datestxt>.
#[derive(Debug, Clone, Deserialize)]
pub struct CalendarDate {
    pub service_id: String,
    pub date: String, // YYYYMMDD
    pub exception_type: ExceptionType,
}

/// A transit agency (operator).
#[derive(Debug, Clone, Deserialize)]
pub struct Agency {
    pub agency_timezone: String,
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
    pub route_type: Option<u16>,
}

impl Route {
    /// Map GTFS `route_type` integer to a display label.
    #[must_use]
    pub fn transport_label(&self) -> &'static str {
        match self.route_type {
            Some(0)                  => "Tram",
            Some(1)                  => "Subway",
            Some(2 | 100..=199) => "Rail",
            Some(3 | 700..=799) => "Bus",
            Some(4)                  => "Ferry",
            Some(5)                  => "Cable Car",
            Some(6)                  => "Gondola",
            Some(7)                  => "Funicular",
            Some(11)                 => "Trolleybus",
            _                        => "Transit",
        }
    }

    /// CSS class name for the route badge (used by the web frontend).
    #[must_use]
    pub fn transport_css_class(&self) -> &'static str {
        Self::transport_css_class_for(self.route_type)
    }

    /// CSS class name from a raw `route_type` value (avoids constructing a [`Route`]).
    #[must_use]
    pub fn transport_css_class_for(route_type: Option<u16>) -> &'static str {
        match route_type {
            Some(0 | 5 | 6 | 7 | 11) => "tram",
            Some(1)                   => "subway",
            Some(2 | 100..=199)       => "rail",
            Some(3 | 700..=799)       => "bus",
            Some(4)                   => "ferry",
            _                         => "transit",
        }
    }
}
