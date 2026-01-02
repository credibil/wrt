cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use wasi_http::{WasiHttp, HttpDefault};
        use wasi_keyvalue::{WasiKeyValue, KeyValueDefault};
        use wasi_otel::{WasiOtel, OtelDefault};

        warp::runtime!({
            main: true,
            hosts: {
                WasiHttp: HttpDefault,
                WasiOtel: OtelDefault,
                WasiKeyValue: KeyValueDefault,
            }
        });
    } else {
        fn main() {}
    }
}
