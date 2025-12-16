use credibil_api::{Handler, Request, Response};
use credibil_error::Error;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wasi_sql::into_json;

use crate::error::ApiError;
use crate::provider::GuestProvider;

/// Result type used across the example
pub type Result<T> = anyhow::Result<T, ApiError>;

async fn handle(
    _owner: &str, _req: EventStoreRequest, provider: &impl GuestProvider,
) -> Result<Response<EventStoreResponse>> {
    const POOL_NAME: &str = "eventstore";

    // Execute query and get results.
    let query =
        "SELECT received_at, event_id, event_time, data FROM az_realtime_gtfs_tu LIMIT 100;"
            .to_string();
    let res = provider
        .query(String::from(POOL_NAME), query, vec![])
        .await
        .map_err(|e| Error::ServerError(format!("query failed: {e:?}")))?;

    let mut rows = into_json(res)
        .map_err(|e| Error::ServerError(format!("failed converting to json: {e:?}")))?;

    if let Some(array) = rows.as_array_mut() {
        for row in array {
            if let Some(value) = row.get_mut("data")
                && let serde_json::Value::String(raw) = value
                && let Ok(parsed) = serde_json::from_str(raw)
            {
                *value = parsed;
            }
        }
    }

    Ok(EventStoreResponse { rows }.into())
}

#[derive(Debug, Clone, Deserialize)]
pub struct EventStoreRequest();

#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct EventStoreResponse {
    pub rows: Value,
}

impl<P: GuestProvider> Handler<EventStoreResponse, P> for Request<EventStoreRequest> {
    type Error = ApiError;

    async fn handle(self, owner: &str, provider: &P) -> Result<Response<EventStoreResponse>> {
        handle(owner, self.body, provider).await
    }
}
