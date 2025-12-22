use chrono::{DateTime, Utc};
use credibil_api::{Handler, Request, Response};
use credibil_error::Error;
use sea_query::Expr;
use serde::{Deserialize, Serialize};
use wasi_sql::types::Row;

use crate::error::ApiError;
use crate::orm::{FetchValue, SelectBuilder, SqlModel, table_column};
use crate::provider::GuestProvider;
use crate::sql_model;

/// Result type used across the example
pub type Result<T> = anyhow::Result<T, ApiError>;

async fn handle(
    _owner: &str, _req: EventStoreRequest, provider: &impl GuestProvider,
) -> Result<Response<EventStoreResponse>> {
    const POOL_NAME: &str = "eventstore";

    let cutoff = Utc::now();
    let events = SelectBuilder::<Event>::new()
        .r#where(Expr::col(table_column(Event::TABLE, "event_time")).gt(cutoff))
        .order_by_desc(None, "event_time")
        .limit(100)
        .fetch_all(provider, POOL_NAME)
        .await
        .map_err(|e| Error::ServerError(format!("failed building query: {e:?}")))?;

    Ok(EventStoreResponse { events }.into())
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventStoreRequest();

#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct EventStoreResponse {
    pub events: Vec<Event>,
}

sql_model!(
    table = "az_realtime_gtfs_tu",
    #[derive(Debug, Clone, Serialize)]
    pub struct Event {
        pub received_at: DateTime<Utc>,
        pub event_id: String,
        pub event_time: DateTime<Utc>,
        pub data: serde_json::Value,
    }
);

impl<P: GuestProvider> Handler<EventStoreResponse, P> for Request<EventStoreRequest> {
    type Error = ApiError;

    async fn handle(self, owner: &str, provider: &P) -> Result<Response<EventStoreResponse>> {
        handle(owner, self.body, provider).await
    }
}
