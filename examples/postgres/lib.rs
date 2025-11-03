//! Minimal example to show a SQL guest that can be used with a Postgres
//! resource.
//!
//! The interface exposes simple HTTP GET and POST endpoints to trigger the SQL
//! queries.

#![allow(missing_docs)]
#![cfg(target_arch = "wasm32")]
use anyhow::anyhow;
use axum::routing::{get, post};
use axum::{Json, Router};
use bytes::Bytes;
use serde_json::{Value, json};
use tracing::Level;
use wasi_http::Result;
use wasi_sql::readwrite;
use wasi_sql::types::{Connection, Statement};
use wasip3::exports::http::handler::Guest;
use wasip3::http::types::{ErrorCode, Request, Response};

struct Http;
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    #[wasi_otel::instrument(name = "http_guest_handle",level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        tracing::debug!("received request: {:?}", request);
        let router = Router::new().route("/", get(select)).route("/", post(insert));
        wasi_http::serve(router, request).await
    }
}

#[axum::debug_handler]
#[wasi_otel::instrument]
async fn select(body: Bytes) -> Result<Json<Value>> {
    tracing::info!("handling request with body: {:?}", body);
    let query = "SELECT * from gtfs.agency;";
    let params = &[];
    let pool =
        Connection::open("postgres").map_err(|e| anyhow!("failed to open connection: {e:?}"))?;

    let stmt = Statement::prepare(query, params)
        .map_err(|e| anyhow!("failed to prepare statement: {e:?}"))?;

    let res = readwrite::query(&pool, &stmt).map_err(|e| anyhow!("query failed: {e:?}"))?;

    let serialized_rows: Vec<String> = res.into_iter().map(|r| format!("{r:?}")).collect();

    Ok(Json(json!({
        "message": serialized_rows
    })))
}

#[axum::debug_handler]
#[wasi_otel::instrument]
async fn insert(body: Bytes) -> Result<Json<Value>> {
    tracing::info!("handling request with body: {:?}", body);
    let query = "insert into gtfs.agency (feed_id, agency_id, agency_name, agency_url, agency_timezone) values ($1, $2, $3, $4, $5);";
    let params: Vec<String> = ["1224", "test1", "name1", "url1", "NZL"]
        .iter()
        .map(std::string::ToString::to_string)
        .collect();

    let pool =
        Connection::open("postgres").map_err(|e| anyhow!("failed to open connection: {e:?}"))?;

    let stmt = Statement::prepare(query, &params)
        .map_err(|e| anyhow!("failed to prepare statement: {e:?}"))?;

    let res = readwrite::exec(&pool, &stmt).map_err(|e| anyhow!("query failed: {e:?}"))?;

    Ok(Json(json!({
        "message": res
    })))
}
