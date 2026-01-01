cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use wasi_config::{WasiConfig, ConfigDefault};
        use wasi_http::{WasiHttp, HttpDefault};
        use wasi_otel::{WasiOtel, OtelDefault};

        warp::runtime!(main, {
            WasiConfig: ConfigDefault,
            WasiHttp: HttpDefault,
            WasiOtel: OtelDefault,
        });
    } else {
        fn main() {}
    }
}
