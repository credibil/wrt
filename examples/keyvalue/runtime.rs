cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use wasi_http::{WasiHttp, WasiHttpCtxImpl as HttpDefault};
        use wasi_keyvalue::{WasiKeyValue, WasiKeyValueCtxImpl as KeyValueDefault};
        use wasi_otel::{WasiOtel, WasiOtelCtxImpl as OtelDefault};

        buildgen::runtime!(main, {
            WasiHttp: HttpDefault,
            WasiOtel: OtelDefault,
            WasiKeyValue: KeyValueDefault,
        });
    } else {
        fn main() {}
    }
}
