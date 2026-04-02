// Field names are dictated by the GTFS CSV specification.
#![allow(clippy::struct_field_names)]

use serde::Deserialize;

/// A GTFS service time, which may exceed 24 hours for post-midnight trips.
///
/// The GTFS spec allows `HH:MM:SS` where HH ≥ 24 to represent times that run
/// past midnight of the service day — e.g. `25:22:00` means 01:22 AM the
/// following calendar day. See <https://gtfs.org/documentation/schedule/reference/#stop_timestxt>.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GtfsTime(u32); // seconds since midnight of the service day

impl GtfsTime {
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
        if h >= 24 {
            // Past midnight of the service day; show wall-clock time with +1d marker.
            let h = h - 24;
            write!(f, "{h:02}:{m:02}+1d")
        } else {
            write!(f, "{h:02}:{m:02}")
        }
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
    pub route_id: String,
    pub trip_id: String,
    #[serde(default)]
    pub trip_headsign: Option<String>,
}

/// A transit route.
#[derive(Debug, Clone, Deserialize)]
pub struct Route {
    pub route_id: String,
    #[serde(default)]
    pub route_short_name: Option<String>,
    #[serde(default)]
    pub route_long_name: Option<String>,
}
