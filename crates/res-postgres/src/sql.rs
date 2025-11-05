use std::sync::Arc;

use anyhow::anyhow;
use chrono::NaiveDate;
use deadpool_postgres::Object;
use futures::future::FutureExt;
use serde_json::json;
use tokio_postgres::row::Row as PgRow;
use tokio_postgres::types::ToSql;
use wasi_sql::{Connection, DataType, FutureResult, Row, WasiSqlCtx};

use crate::Client;

type Param = Box<dyn ToSql + Send + Sync>;
type ParamRef<'a> = &'a (dyn ToSql + Sync);

impl WasiSqlCtx for Client {
    fn open(&self, name: String) -> FutureResult<Arc<dyn Connection>> {
        tracing::debug!("getting connection {name}");
        let pool = self.0.clone();

        async move {
            let cnn = pool.get().await.map_err(|e| anyhow!("issue getting connection: {e}"))?;
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
            let pg_params = params.iter().map(|s| into_param(s)).collect::<Vec<_>>();
            let param_refs: Vec<ParamRef> =
                pg_params.iter().map(|b| b.as_ref() as ParamRef).collect();
            let pg_rows =
                cnn.query(&query, &param_refs).await.map_err(|e| anyhow!("query failed: {e}"))?;

            let wasi_rows = pg_rows
                .iter()
                .enumerate()
                .flat_map(|(idx, row)| into_wasi_row(row, idx))
                .collect::<Vec<_>>();

            Ok(wasi_rows)
        }
        .boxed()
    }

    fn exec(&self, query: String, params: Vec<DataType>) -> FutureResult<u32> {
        tracing::debug!("exec: {query}, params: {params:?}");
        let cnn = Arc::clone(&self.0);

        async move {
            let pg_params = params.iter().map(|s| into_param(s)).collect::<Vec<_>>();
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
// There is not a one-to-one mapping between `wasi-sql` `DataType` and Postgres
// types, so the `WIT` may need to be expanded in future to support more types.
fn into_param(value: &DataType) -> anyhow::Result<Param> {
    match value {
        DataType::Int32(i) => Ok(Box::new(*i) as Param), // INT, SERIAL
        DataType::Int64(i) => Ok(Box::new(*i) as Param), // BIGINT, BIGSERIAL
        DataType::Uint32(u) => Ok(Box::new(*u) as Param), // OID
        DataType::Uint64(_u) => Err(anyhow!("no Postgres equivalent for uint64")),
        DataType::Float(f) => Ok(Box::new(*f) as Param), // REAL
        DataType::Double(d) => Ok(Box::new(*d) as Param), // DOUBLE PRECISION
        DataType::Str(s) => Ok(Box::new(s.clone()) as Param), // TEXT, VARCHAR, CHAR(n), CITEXT, NAME
        DataType::Boolean(b) => Ok(Box::new(*b) as Param),    // BOOL
        DataType::Date(fv) => match fv {
            Some(fv) => match NaiveDate::parse_from_str(&fv.value, &fv.format) {
                Ok(d) => Ok(Box::new(d) as Param),
                Err(e) => return Err(anyhow!("invalid date format: {e}")),
            },
            None => Ok(Box::new(None::<NaiveDate>) as Param),
        }, // DATE
        DataType::Time(fv) => match fv {
            Some(fv) => match chrono::NaiveTime::parse_from_str(&fv.value, &fv.format) {
                Ok(t) => Ok(Box::new(t) as Param),
                Err(e) => return Err(anyhow!("invalid time format: {e}")),
            },
            None => Ok(Box::new(None::<chrono::NaiveTime>) as Param),
        }, // TIME
        DataType::Timestamp(fv) => match fv {
            Some(fv) => match chrono::NaiveDateTime::parse_from_str(&fv.value, &fv.format) {
                Ok(ts) => Ok(Box::new(ts) as Param),
                Err(e) => return Err(anyhow!("invalid timestamp format: {e}")),
            },
            None => Ok(Box::new(None::<chrono::NaiveDateTime>) as Param),
        } // TIMESTAMP
        DataType::Binary(v) => Ok(Box::new(v.clone()) as Param), // BYTEA
    }
}

// Converts a Postgres row into a `wasi-sql` `Row`.
fn into_wasi_row(pg_row: &PgRow, idx: usize) -> Vec<Row> {
    let mut row_data = serde_json::Map::new();

    for (i, col) in pg_row.columns().iter().enumerate() {
        let col_name = col.name().to_string();
        let value = pg_row.try_get::<usize, String>(i).ok();
        row_data.insert(col_name, value.map_or_else(|| json!(null), |v| json!(v)));
    }

    let row_json = serde_json::to_string(&row_data).unwrap_or_else(|_| "{}".to_string());

    vec![Row {
        field_name: idx.to_string(),
        value: DataType::Str(row_json),
    }]
}
