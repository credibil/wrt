use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use wasi_sql::types::Row;

use crate::entity;
use crate::orm::{Entity, FetchValue};

#[derive(Deserialize)]
pub struct GetEventsParams {
    pub from: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetEventsRequest {
    pub limit: i32,
    pub from: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct GetEventsResponse {
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveEventRequest {
    pub event_id: String,
    pub event_time: DateTime<Utc>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct SaveEventResponse {
    pub event: Event,
}

entity!(
    table = "az_realtime_gtfs_tu",
    #[derive(Debug, Clone, Serialize)]
    pub struct Event {
        pub received_at: DateTime<Utc>,
        pub event_id: String,
        pub event_time: DateTime<Utc>,
        pub data: serde_json::Value,
    }
);
