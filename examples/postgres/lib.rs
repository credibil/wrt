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
use serde_json::{Map, Number, Value, json};
use tracing::Level;
use wasi_http::Result;
use wasi_sql::readwrite;
use wasi_sql::types::{Connection, DataType, Statement};
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
    let query = "SELECT * from mytable;";
    let params = &[];
    let pool =
        Connection::open("postgres").map_err(|e| anyhow!("failed to open connection: {e:?}"))?;

    let stmt = Statement::prepare(query, params)
        .map_err(|e| anyhow!("failed to prepare statement: {e:?}"))?;

    let res = readwrite::query(&pool, &stmt).map_err(|e| anyhow!("query failed: {e:?}"))?;

    let mut json_map = Map::new();
    for row in &res {
        let key = &row.field_name;
        let value = match &row.value {
            DataType::Int32(v) => Value::Number((*v).into()),
            DataType::Int64(v) => Value::Number((*v).into()),
            DataType::Uint32(v) => Value::Number((*v).into()),
            DataType::Uint64(v) => Value::Number((*v).into()),
            DataType::Float(v) | DataType::Double(v) => {
                if v.is_nan() {
                    Value::String("NaN".to_string())
                } else if v.is_infinite() && *v > 0.0 {
                    Value::String("Infinity".to_string())
                } else if v.is_infinite() && *v < 0.0 {
                    Value::String("-Infinity".to_string())
                } else {
                    Value::Number(Number::from_f64(*v).unwrap_or_else(|| Number::from(0)))
                }
            }
            DataType::Str(v) | DataType::Date(v) | DataType::Time(v) | DataType::Timestamp(v) => {
                Value::String(v.clone())
            }
            DataType::Boolean(v) => Value::Bool(*v),
            _ => Value::String(format!("{:?}", &row.value)),
        };
        json_map.insert(key.clone(), value);
    }
    let serialized_rows = Value::Object(json_map);

    Ok(Json(serialized_rows))
}

#[axum::debug_handler]
#[wasi_otel::instrument]
async fn insert(body: Bytes) -> Result<Json<Value>> {
    tracing::info!("handling request with body: {:?}", body);
    let query = "insert into mytable (feed_id, agency_id, agency_name, agency_url, agency_timezone) values ($1, $2, $3, $4, $5);";
    let params: Vec<String> = ["1224", "test1", "name1", "url1", "NZL"]
        .iter()
        .map(std::string::ToString::to_string)
        .collect();

    tracing::debug!("opening connection to postgres");
    let pool =
        Connection::open("postgres").map_err(|e| anyhow!("failed to open connection: {e:?}"))?;
    tracing::debug!("preparing statement");
    let stmt = Statement::prepare(query, &params)
        .map_err(|e| anyhow!("failed to prepare statement: {e:?}"))?;

    let res = readwrite::exec(&pool, &stmt).map_err(|e| anyhow!("query failed: {e:?}"))?;

    Ok(Json(json!({
        "message": res
    })))
}
