#![cfg(not(target_arch = "wasm32"))]

use be_postgres::Client as Postgres;
use wasi_http::{WasiHttp, WasiHttpCtxImpl as HttpDefault};
use wasi_otel::{WasiOtel, WasiOtelCtxImpl as OtelDefault};
use wasi_sql::WasiSql;

buildgen::runtime!(main, {
    WasiHttp: HttpDefault,
    WasiOtel: OtelDefault,
    WasiSql: Postgres,
});
