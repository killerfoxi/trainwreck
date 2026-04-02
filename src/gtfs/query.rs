use std::collections::HashMap;

use super::model::{Route, StopTime, Trip};

/// All trips serving a set of stops, with their routes resolved.
pub struct StopSchedule {
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

        deps.sort_unstable_by_key(|&(st, _, _)| st.departure_time);
        deps
    }

}
