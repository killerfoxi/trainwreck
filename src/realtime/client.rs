use prost::Message;

use super::{
    error::RealtimeError,
    model::{RealtimeFeed, StopTimeStatus, TripStatus},
    proto::FeedMessage,
};

const URL: &str = "https://api.opentransportdata.swiss/la/gtfs-rt";

// TripDescriptor.ScheduleRelationship::CANCELED = 3
const TRIP_CANCELED: i32 = 3;
// StopTimeUpdate.ScheduleRelationship::SKIPPED = 1
const STOP_SKIPPED: i32 = 1;

pub async fn fetch_trip_updates(api_key: &str) -> Result<RealtimeFeed, RealtimeError> {
    let bytes = reqwest::Client::new()
        .get(URL)
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;

    let feed = FeedMessage::decode(&bytes[..])?;

    let trips = feed
        .entity
        .into_iter()
        .filter_map(|entity| entity.trip_update)
        .filter_map(|tu| {
            let trip_id = tu.trip.trip_id.filter(|id| !id.is_empty())?;

            let status = if tu.trip.schedule_relationship == Some(TRIP_CANCELED) {
                TripStatus::Canceled
            } else {
                let stops = tu
                    .stop_time_update
                    .into_iter()
                    .filter_map(|stu| {
                        let stop_id = stu.stop_id.filter(|id| !id.is_empty())?;
                        Some((
                            stop_id,
                            StopTimeStatus {
                                departure_delay_secs: stu.departure.and_then(|e| e.delay),
                                skipped: stu.schedule_relationship == Some(STOP_SKIPPED),
                            },
                        ))
                    })
                    .collect();
                TripStatus::Running(stops)
            };

            Some((trip_id, status))
        })
        .collect();

    Ok(RealtimeFeed { trips })
}
