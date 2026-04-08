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

/// A service schedule defining which days a service operates.
/// See <https://gtfs.org/documentation/schedule/reference/#calendartxt>.
#[derive(Debug, Clone, Deserialize)]
pub struct Calendar {
    pub service_id: String,
    pub monday: u8,
    pub tuesday: u8,
    pub wednesday: u8,
    pub thursday: u8,
    pub friday: u8,
    pub saturday: u8,
    pub sunday: u8,
    pub start_date: String, // YYYYMMDD
    pub end_date: String,   // YYYYMMDD
}

impl Calendar {
    pub fn runs_on(&self, weekday: Weekday) -> bool {
        match weekday {
            Weekday::Monday => self.monday == 1,
            Weekday::Tuesday => self.tuesday == 1,
            Weekday::Wednesday => self.wednesday == 1,
            Weekday::Thursday => self.thursday == 1,
            Weekday::Friday => self.friday == 1,
            Weekday::Saturday => self.saturday == 1,
            Weekday::Sunday => self.sunday == 1,
        }
    }
}

/// A service date exception (added or removed service day).
/// See <https://gtfs.org/documentation/schedule/reference/#calendar_datestxt>.
#[derive(Debug, Clone, Deserialize)]
pub struct CalendarDate {
    pub service_id: String,
    pub date: String,       // YYYYMMDD
    pub exception_type: u8, // 1 = added, 2 = removed
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
    /// Map GTFS route_type integer to a display label.
    pub fn transport_label(&self) -> &'static str {
        match self.route_type {
            Some(0)        => "Tram",
            Some(1)        => "Subway",
            Some(2)        => "Rail",
            Some(3)        => "Bus",
            Some(4)        => "Ferry",
            Some(5)        => "Cable Car",
            Some(6)        => "Gondola",
            Some(7)        => "Funicular",
            Some(11)       => "Trolleybus",
            Some(100..=199)=> "Rail",
            Some(700..=799)=> "Bus",
            _              => "Transit",
        }
    }

    /// CSS class name for the route badge (used by the web frontend).
    pub fn transport_css_class(&self) -> &'static str {
        Self::transport_css_class_for(self.route_type)
    }

    /// CSS class name from a raw `route_type` value (avoids constructing a Route).
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
