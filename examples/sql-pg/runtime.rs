cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use wasi_http::{WasiHttp, WasiHttpCtxImpl as HttpDefault};
        use wasi_otel::{WasiOtel, WasiOtelCtxImpl as OtelDefault};
        use wasi_sql::WasiSql;
        use be_postgres::Client as SqlBackend;

        buildgen::runtime!(main, {
            WasiHttp: HttpDefault,
            WasiOtel: OtelDefault,
            WasiSql: SqlBackend,
        });
    } else {
        fn main() {}
    }
}
