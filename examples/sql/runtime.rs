cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use wasi_http::{WasiHttp, WasiHttpCtxImpl as HttpDefault};
        use wasi_otel::{WasiOtel, WasiOtelCtxImpl as OtelDefault};
        use wasi_sql::{WasiSql, WasiSqlCtxImpl as SqlDefault};

        buildgen::runtime!(main, {
            WasiHttp: HttpDefault,
            WasiOtel: OtelDefault,
            WasiSql: SqlDefault,
        });
    } else {
        fn main() {}
    }
}
