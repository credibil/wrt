cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use wasi_http::{WasiHttp, HttpDefault};
        use wasi_otel::{WasiOtel, OtelDefault};
        use wasi_sql::{WasiSql, SqlDefault};

        warp::runtime!({
            main: true,
            hosts: {
                WasiHttp: HttpDefault,
                WasiOtel: OtelDefault,
                WasiSql: SqlDefault,
            }
        });
    } else {
        fn main() {}
    }
}
