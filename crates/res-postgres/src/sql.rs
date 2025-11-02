use std::sync::Arc;

use anyhow::anyhow;
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
        tracing::trace!("getting connection {name}");
        let pool = self.0.clone();

        async move {
            let cnn = pool.get().await.map_err(|e| anyhow!("issue getting connection: {e}"))?;
            Ok(Arc::new(PostgresConnection(Arc::new(cnn))) as Arc<dyn Connection>)
        }
        .boxed()
    }
}

#[derive(Debug)]
pub struct PostgresConnection(Arc<Object>);

impl Connection for PostgresConnection {
    fn query(&self, query: String, params: Vec<String>) -> FutureResult<Vec<Row>> {
        tracing::trace!("query: {query}, params: {params:?}");
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

    fn exec(&self, query: String, params: Vec<String>) -> FutureResult<u32> {
        tracing::trace!("exec: {query}, params: {params:?}");
        let cnn = Arc::clone(&self.0);

        async move {
            let pg_params = params.iter().map(|s| into_param(s)).collect::<Vec<_>>();
            let param_refs: Vec<ParamRef> =
                pg_params.iter().map(|b| b.as_ref() as ParamRef).collect();
            let affected = cnn.execute(&query, &param_refs).await?;

            #[allow(clippy::cast_possible_truncation)]
            Ok(affected as u32)
        }
        .boxed()
    }
}

// Parses a string parameter into a Postgres-compatible type.
fn into_param(value: &str) -> Param {
    value
        .parse::<i32>()
        .map(|i| Box::new(i) as Param)
        .or_else(|_| value.parse::<i64>().map(|ts| Box::new(ts) as Param))
        .or_else(|_| value.parse::<f64>().map(|f| Box::new(f) as Param))
        .or_else(|_| value.parse::<bool>().map(|b| Box::new(b) as Param))
        .unwrap_or_else(|_| Box::new(value.to_owned()) as Param)
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
