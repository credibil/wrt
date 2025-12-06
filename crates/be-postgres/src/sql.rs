use std::sync::Arc;

use anyhow::{Context, anyhow, bail};
use chrono::NaiveDate;
use deadpool_postgres::Object;
use futures::future::FutureExt;
use tokio_postgres::row::Row as PgRow;
use tokio_postgres::types::ToSql;
use wasi_sql::{Connection, DataType, Field, FormattedValue, FutureResult, Row, WasiSqlCtx};

use crate::Client;

type Param = Box<dyn ToSql + Send + Sync>;
type ParamRef<'a> = &'a (dyn ToSql + Sync);

impl WasiSqlCtx for Client {
    fn open(&self, name: String) -> FutureResult<Arc<dyn Connection>> {
        tracing::debug!("getting connection {name}");
        let pool = self.0.clone();

        async move {
            let cnn = pool.get().await.context("issue getting connection")?;
            Ok(Arc::new(PostgresConnection(Arc::new(cnn))) as Arc<dyn Connection>)
        }
        .boxed()
    }
}

#[derive(Debug)]
struct PostgresConnection(Arc<Object>);

impl Connection for PostgresConnection {
    fn query(&self, query: String, params: Vec<DataType>) -> FutureResult<Vec<Row>> {
        tracing::debug!("query: {query}, params: {params:?}");
        let cnn = Arc::clone(&self.0);

        async move {
            let mut pg_params: Vec<Param> = Vec::new();
            for p in &params {
                pg_params.push(into_param(p)?);
            }
            let param_refs: Vec<ParamRef> =
                pg_params.iter().map(|b| b.as_ref() as ParamRef).collect();

            let pg_rows =
                cnn.query(&query, &param_refs).await.context("query failed")?;
            tracing::debug!("query returned {} rows", pg_rows.len());

            let mut wasi_rows = Vec::new();
            for (idx, r) in pg_rows.iter().enumerate() {
                let row = match into_wasi_row(r, idx) {
                    Ok(row) => row,
                    Err(e) => {
                        tracing::error!("failed to convert row: {e:?}");
                        return Err(anyhow!("failed to convert row: {e:?}"));
                    }
                };
                wasi_rows.push(row);
            }

            Ok(wasi_rows)
        }
        .boxed()
    }

    fn exec(&self, query: String, params: Vec<DataType>) -> FutureResult<u32> {
        tracing::debug!("exec: {query}, params: {params:?}");
        let cnn = Arc::clone(&self.0);

        async move {
            let mut pg_params: Vec<Param> = Vec::new();
            for p in &params {
                pg_params.push(into_param(p)?);
            }
            let param_refs: Vec<ParamRef> =
                pg_params.iter().map(|b| b.as_ref() as ParamRef).collect();

            let affected = match cnn.execute(&query, &param_refs).await {
                Ok(count) => count,
                Err(e) => {
                    tracing::error!("exec failed: {e}");
                    return Err(anyhow!("exec failed: {e}"));
                }
            };
            #[allow(clippy::cast_possible_truncation)]
            Ok(affected as u32)
        }
        .boxed()
    }
}

// Parses a string parameter into a Postgres-compatible type.
//
// Note: Postgres has a huge variety of types - many more than are represented
// in `wasi-sql::DataType`. The `WIT` may need to be expanded in future to
// support more types, and this function updated accordingly.
fn into_param(value: &DataType) -> anyhow::Result<Param> {
    match value {
        DataType::Int32(i) => Ok(Box::new(*i) as Param), // INT, SERIAL
        DataType::Int64(i) => Ok(Box::new(*i) as Param), // BIGINT, BIGSERIAL
        DataType::Uint32(u) => Ok(Box::new(*u) as Param), // OID
        DataType::Uint64(_u) => Err(anyhow!("no Postgres equivalent for uint64")),
        DataType::Float(f) => Ok(Box::new(*f) as Param), // REAL
        DataType::Double(d) => Ok(Box::new(*d) as Param), // DOUBLE PRECISION
        DataType::Str(s) => Ok(Box::new(s.clone()) as Param), // TEXT, VARCHAR, CHAR(n), NAME
        DataType::Boolean(b) => Ok(Box::new(*b) as Param), // BOOL
        DataType::Date(fv) => fv.as_ref().map_or_else(
            || Ok(Box::new(None::<NaiveDate>) as Param),
            |fv| match NaiveDate::parse_from_str(&fv.value, &fv.format) {
                Ok(d) => Ok(Box::new(d) as Param),
                Err(e) => Err(anyhow!("invalid date format: {e}")),
            },
        ), // DATE
        DataType::Time(fv) => fv.as_ref().map_or_else(
            || Ok(Box::new(None::<chrono::NaiveTime>) as Param),
            |fv| match chrono::NaiveTime::parse_from_str(&fv.value, &fv.format) {
                Ok(t) => Ok(Box::new(t) as Param),
                Err(e) => Err(anyhow!("invalid time format: {e}")),
            },
        ), // TIME
        DataType::Timestamp(fv) => fv.as_ref().map_or_else(
            || Ok(Box::new(None::<chrono::NaiveDateTime>) as Param),
            |fv| match chrono::NaiveDateTime::parse_from_str(&fv.value, &fv.format) {
                Ok(ts) => Ok(Box::new(ts) as Param),
                Err(e) => Err(anyhow!("invalid timestamp format: {e}")),
            },
        ), // TIMESTAMP
        DataType::Binary(v) => Ok(Box::new(v.clone()) as Param), // BYTEA
    }
}

// Converts a Postgres row into a `wasi-sql` `Row`.
//
//
// Note: Postgres has a huge variety of types - many more than are represented
// in `wasi-sql::DataType`. The `WIT` may need to be expanded in future to
// support more types, and this function updated accordingly.
fn into_wasi_row(pg_row: &PgRow, idx: usize) -> anyhow::Result<Row> {
    let mut fields = Vec::new();
    for (i, col) in pg_row.columns().iter().enumerate() {
        let name = col.name().to_string();
        tracing::debug!("attempting to convert column '{name}' with type '{:?}'", col.type_());
        tracing::debug!("column type name: {}", col.type_().name());
        let value = match col.type_().name() {
            "int4" => {
                let v: Option<i32> = pg_row.try_get(i)?;
                DataType::Int32(v)
            }
            "int8" => {
                let v: Option<i64> = pg_row.try_get(i)?;
                DataType::Int64(v)
            }
            "oid" => {
                let v: Option<u32> = pg_row.try_get(i)?;
                DataType::Uint32(v)
            }
            "float4" => {
                let v: Option<f32> = pg_row.try_get(i)?;
                DataType::Float(v)
            }
            "float8" => {
                let v: Option<f64> = pg_row.try_get(i)?;
                DataType::Double(v)
            }
            "text" | "varchar" | "name" | "_char" => {
                let v: Option<String> = pg_row.try_get(i)?;
                DataType::Str(v)
            }
            "bool" => {
                let v: Option<bool> = pg_row.try_get(i)?;
                DataType::Boolean(v)
            }
            "date" => {
                let v: Option<NaiveDate> = pg_row.try_get(i)?;
                let formatted = v.map(|date| date.format("%Y-%m-%d").to_string());
                DataType::Date(formatted.map(|value| FormattedValue {
                    value,
                    format: "%Y-%m-%d".to_string(),
                }))
            }
            "time" => {
                let v: Option<chrono::NaiveTime> = pg_row.try_get(i)?;
                let formatted = v.map(|time| time.format("%H:%M:%S").to_string());
                DataType::Time(formatted.map(|value| FormattedValue {
                    value,
                    format: "%H:%M:%S".to_string(),
                }))
            }
            "timestamp" => {
                let v: Option<chrono::NaiveDateTime> = pg_row.try_get(i)?;
                // Use explicit format compatible with NaiveDateTime (no timezone)
                let format_str = "%Y-%m-%dT%H:%M:%S";
                let formatted = v.map(|dt| dt.format(format_str).to_string());
                DataType::Timestamp(formatted.map(|value| FormattedValue {
                    value,
                    format: format_str.to_string(),
                }))
            }
            "bytea" => {
                let v: Option<Vec<u8>> = pg_row.try_get(i)?;
                DataType::Binary(v)
            }
            other => {
                bail!("unsupported column type: {other}");
            }
        };
        tracing::debug!("converted column '{name}' to value '{:?}'", value);
        fields.push(Field { name, value });
    }

    Ok(Row {
        index: idx.to_string(),
        fields,
    })
}
