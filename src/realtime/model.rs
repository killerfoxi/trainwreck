use std::collections::HashMap;

pub struct StopTimeStatus {
    pub departure_delay_secs: Option<i32>,
    pub skipped: bool,
}

pub enum TripStatus {
    Running(HashMap<String, StopTimeStatus>),
    Canceled,
}

pub struct RealtimeFeed {
    pub trips: HashMap<String, TripStatus>,
}

pub enum DepartureStatus {
    OnTime { delay_secs: i32 },
    Canceled,
    Skipped,
}

impl std::fmt::Display for DepartureStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Canceled => write!(f, "[CANCELLED]"),
            Self::Skipped => write!(f, "[STOP SKIPPED]"),
            Self::OnTime { delay_secs: 0 } => write!(f, "[on time]"),
            Self::OnTime { delay_secs } => {
                let sign = if *delay_secs > 0 { "+" } else { "-" };
                let secs = delay_secs.unsigned_abs();
                let (mins, rem) = (secs / 60, secs % 60);
                if mins == 0 {
                    return write!(f, "[{sign}{rem}s]");
                }
                write!(f, "[{sign}{mins}m{rem:02}s]")
            }
        }
    }
}

impl RealtimeFeed {
    pub fn status_for(&self, trip_id: &str, stop_id: &str) -> Option<DepartureStatus> {
        match self.trips.get(trip_id)? {
            TripStatus::Canceled => Some(DepartureStatus::Canceled),
            TripStatus::Running(stops) => {
                let sts = stops.get(stop_id)?;
                if sts.skipped {
                    return Some(DepartureStatus::Skipped);
                }
                Some(DepartureStatus::OnTime {
                    delay_secs: sts.departure_delay_secs.unwrap_or(0),
                })
            }
        }
    }
}
