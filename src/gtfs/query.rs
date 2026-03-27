use std::collections::HashMap;

use super::model::{Route, Stop, StopTime, Trip};

/// All trips serving a particular stop, with their routes resolved.
pub struct StopSchedule {
    pub stop: Stop,
    pub stop_times: Vec<StopTime>,
    pub trips: HashMap<String, Trip>,
    pub routes: HashMap<String, Route>,
}

impl StopSchedule {
    /// Iterate over departures sorted by departure time,
    /// yielding `(stop_time, trip, route)` tuples.
    pub fn departures(&self) -> Vec<(&StopTime, &Trip, Option<&Route>)> {
        let mut deps: Vec<_> = self
            .stop_times
            .iter()
            .filter_map(|st| {
                let trip = self.trips.get(&st.trip_id)?;
                let route = self.routes.get(&trip.route_id);
                Some((st, trip, route))
            })
            .collect();

        deps.sort_unstable_by(|a, b| a.0.departure_time.cmp(&b.0.departure_time));
        deps
    }

    /// Unique route summaries serving this stop.
    pub fn route_summaries(&self) -> Vec<RouteSummary> {
        let mut seen = HashMap::new();

        for (st, trip, route) in self.departures() {
            seen.entry(trip.route_id.clone())
                .and_modify(|s: &mut RouteSummary| s.trip_count += 1)
                .or_insert(RouteSummary {
                    route_id: trip.route_id.clone(),
                    short_name: route.and_then(|r| r.route_short_name.clone()),
                    long_name: route.and_then(|r| r.route_long_name.clone()),
                    route_type: route.and_then(|r| r.route_type),
                    headsign: trip.trip_headsign.clone(),
                    first_departure: st.departure_time.clone(),
                    trip_count: 1,
                });
        }

        let mut summaries: Vec<_> = seen.into_values().collect();
        summaries.sort_unstable_by(|a, b| a.first_departure.cmp(&b.first_departure));
        summaries
    }
}

/// Condensed view of a route serving a stop.
pub struct RouteSummary {
    pub route_id: String,
    pub short_name: Option<String>,
    pub long_name: Option<String>,
    pub route_type: Option<super::model::RouteType>,
    pub headsign: Option<String>,
    pub first_departure: String,
    pub trip_count: usize,
}

impl std::fmt::Display for RouteSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = self
            .short_name
            .as_deref()
            .or(self.long_name.as_deref())
            .unwrap_or(&self.route_id);

        write!(f, "{name}")?;

        if let Some(headsign) = &self.headsign {
            write!(f, " → {headsign}")?;
        }

        if let Some(rt) = self.route_type {
            write!(f, " ({rt})")?;
        }

        write!(f, " — {} trips, first at {}", self.trip_count, self.first_departure)
    }
}
