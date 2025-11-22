#![cfg(target_arch = "wasm32")]

//! Minimal example to show a SQL guest that can be used with a Postgres
//! resource.
//!
//! The interface exposes simple HTTP GET and POST endpoints to trigger the SQL
//! queries.

use anyhow::anyhow;
use axum::routing::{get, post};
use axum::{Json, Router};
use bytes::Bytes;
use serde_json::{Value, json};
use tracing::Level;
use wasi_http::Result;
use wasi_sql::types::{Connection, DataType, FormattedValue, Statement};
use wasi_sql::{into_json, readwrite};
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    #[wasi_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        tracing::debug!("received request: {:?}", request);
        let router = Router::new().route("/", get(query)).route("/", post(insert));
        wasi_http::serve(router, request).await
    }
}

#[axum::debug_handler]
#[wasi_otel::instrument]
async fn query() -> Result<Json<Value>> {
    tracing::info!("query database");

    let pool =
        Connection::open("postgres").map_err(|e| anyhow!("failed to open connection: {e:?}"))?;
    let stmt = Statement::prepare("SELECT * from mytable;", &[])
        .map_err(|e| anyhow!("failed to prepare statement: {e:?}"))?;

    let res = readwrite::query(&pool, &stmt).map_err(|e| anyhow!("query failed: {e:?}"))?;

    Ok(Json(into_json(res)?))
}

#[axum::debug_handler]
#[wasi_otel::instrument]
async fn insert(body: Bytes) -> Result<Json<Value>> {
    tracing::info!("insert data");

    let insert = "insert into mytable (feed_id, agency_id, agency_name, agency_url, agency_timezone, created_at) values ($1, $2, $3, $4, $5, $6);";
    let params: Vec<DataType> = [
        DataType::Int32(Some(1224)),
        DataType::Str(Some("test1".to_string())),
        DataType::Str(Some("name1".to_string())),
        DataType::Str(Some("url1".to_string())),
        DataType::Str(Some("NZL".to_string())),
        DataType::Timestamp(Some(FormattedValue {
            value: "2025-11-06T00:05:30".to_string(),
            format: "%Y-%m-%dT%H:%M:%S".to_string(),
        })),
    ]
    .to_vec();

    tracing::debug!("opening connection");

    let pool =
        Connection::open("db").map_err(|e| anyhow!("failed to open connection: {e:?}"))?;
    let stmt = Statement::prepare(insert, &params)
        .map_err(|e| anyhow!("failed to prepare statement: {e:?}"))?;

    let res = readwrite::exec(&pool, &stmt).map_err(|e| anyhow!("query failed: {e:?}"))?;

    Ok(Json(json!({
        "message": res
    })))
}
