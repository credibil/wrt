use std::str::FromStr;

use chrono::{NaiveDate, NaiveTime, Utc};
use credibil_api::{Handler, Request, Response};
use credibil_error::Error;
use sea_query::Expr;
use serde_json::to_string;

use super::Result;
use crate::error::ApiError;
use crate::handlers::types::{Event, GetEventsRequest, GetEventsResponse};
use crate::handlers::{SaveEventRequest, SaveEventResponse};
use crate::orm::{Entity, InsertBuilder, SelectBuilder, table_column};
use crate::provider::GuestProvider;

async fn get_events(
    _owner: &str, req: GetEventsRequest, provider: &impl GuestProvider,
) -> Result<Response<GetEventsResponse>> {
    const POOL_NAME: &str = "eventstore";

    let from = req
        .from
        .as_deref()
        .and_then(|s| NaiveDate::from_str(s).ok())
        .map(|d| d.and_time(NaiveTime::MIN).and_utc())
        .unwrap_or_else(Utc::now);

    let events = SelectBuilder::<Event>::new()
        .r#where(Expr::col(table_column(Event::TABLE, "event_time")).gt(from))
        .order_by_desc(None, "event_time")
        .limit(req.limit as u64)
        .fetch(provider, POOL_NAME)
        .await
        .map_err(|e| Error::ServerError(format!("failed building query: {e:?}")))?;

    Ok(GetEventsResponse { events }.into())
}

async fn save_event(
    _owner: &str, req: SaveEventRequest, provider: &impl GuestProvider,
) -> Result<Response<SaveEventResponse>> {
    const POOL_NAME: &str = "eventstore";
    let SaveEventRequest {
        event_id,
        event_time,
        data,
    } = req;
    let received_at = Utc::now();

    let payload = to_string(&data)
        .map_err(|e| Error::ServerError(format!("failed to serialize event payload: {e}")))?;

    let query = InsertBuilder::<Event>::new()
        .set("received_at", received_at)
        .set("event_id", event_id.clone())
        .set("event_time", event_time)
        .set("data", payload)
        .build()
        .map_err(|e| Error::ServerError(format!("failed building insert: {e:?}")))?;

    let affected = provider
        .exec(POOL_NAME.to_string(), query.sql, query.params)
        .await
        .map_err(|e| Error::ServerError(format!("insert failed: {e:?}")))?;

    if affected != 1 {
        return Err(Error::ServerError(format!("expected to insert 1 row, got {affected}")).into());
    }

    let event = Event {
        received_at,
        event_id,
        event_time,
        data,
    };

    Ok(SaveEventResponse { event }.into())
}

impl<P: GuestProvider> Handler<GetEventsResponse, P> for Request<GetEventsRequest> {
    type Error = ApiError;

    async fn handle(self, owner: &str, provider: &P) -> Result<Response<GetEventsResponse>> {
        get_events(owner, self.body, provider).await
    }
}

impl<P: GuestProvider> Handler<SaveEventResponse, P> for Request<SaveEventRequest> {
    type Error = ApiError;

    async fn handle(self, owner: &str, provider: &P) -> Result<Response<SaveEventResponse>> {
        save_event(owner, self.body, provider).await
    }
}
