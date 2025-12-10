//! # SQL Guest Module (Default Backend)
//!
//! This module demonstrates the WASI SQL interface with the default backend.
//! It shows how to perform database operations that work with any SQL-compatible
//! database configured by the host.
//!
//! ## Operations Demonstrated
//!
//! - Opening database connections by name
//! - Preparing parameterized SQL statements
//! - Executing SELECT queries
//! - Executing INSERT/UPDATE/DELETE commands
//! - Converting results to JSON
//!
//! ## Security
//!
//! Always use parameterized queries (`$1`, `$2`, etc.) to prevent SQL injection.
//! Never concatenate user input into SQL strings.
//!
//! ## Backend Agnostic
//!
//! This guest code works with any WASI SQL backend:
//! - PostgreSQL (sql-postgres example)
//! - Azure SQL
//! - Any SQL-compatible database

#![cfg(target_arch = "wasm32")]

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

/// HTTP handler struct.
struct Http;

/// Export the HTTP handler for the WASI runtime.
wasip3::http::proxy::export!(Http);

impl Guest for Http {
    /// Routes HTTP requests to database operations.
    ///
    /// - `GET /`: Query all rows from the sample table
    /// - `POST /`: Insert a new row into the sample table
    #[wasi_otel::instrument(name = "http_guest_handle", level = Level::DEBUG)]
    async fn handle(request: Request) -> Result<Response, ErrorCode> {
        tracing::debug!("received request: {:?}", request);
        let router = Router::new().route("/", get(query)).route("/", post(insert));
        wasi_http::serve(router, request).await
    }
}

/// Query all rows from the sample table.
///
/// Demonstrates:
/// - Opening a named database connection
/// - Preparing a SELECT statement
/// - Executing a query and converting results to JSON
#[axum::debug_handler]
#[wasi_otel::instrument]
async fn query() -> Result<Json<Value>> {
    tracing::info!("query database");

    // Open connection using named pool from host configuration.
    let pool =
        Connection::open("postgres").map_err(|e| anyhow!("failed to open connection: {e:?}"))?;

    // Prepare a SELECT statement (no parameters needed).
    let stmt = Statement::prepare("SELECT * from mytable;", &[])
        .map_err(|e| anyhow!("failed to prepare statement: {e:?}"))?;

    // Execute query and get results.
    let res = readwrite::query(&pool, &stmt).map_err(|e| anyhow!("query failed: {e:?}"))?;

    // Convert to JSON for HTTP response.
    Ok(Json(into_json(res)?))
}

/// Insert a new row into the sample table.
///
/// Demonstrates:
/// - Parameterized INSERT statements
/// - Various DataType variants (Int32, Str, Timestamp)
/// - Using exec() for non-SELECT statements
#[axum::debug_handler]
#[wasi_otel::instrument]
async fn insert(_body: Bytes) -> Result<Json<Value>> {
    tracing::info!("insert data");

    // Parameterized INSERT - use $1, $2, etc. for placeholders.
    let insert = "insert into mytable (feed_id, agency_id, agency_name, agency_url, agency_timezone, created_at) values ($1, $2, $3, $4, $5, $6);";

    // Define parameters with their types.
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

    let pool = Connection::open("db").map_err(|e| anyhow!("failed to open connection: {e:?}"))?;
    let stmt = Statement::prepare(insert, &params)
        .map_err(|e| anyhow!("failed to prepare statement: {e:?}"))?;

    // Use exec() for INSERT/UPDATE/DELETE (returns affected row count).
    let res = readwrite::exec(&pool, &stmt).map_err(|e| anyhow!("query failed: {e:?}"))?;

    Ok(Json(json!({
        "message": res
    })))
}
