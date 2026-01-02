cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use wasi_http::{WasiHttp, HttpDefault};
        use wasi_otel::{WasiOtel, OtelDefault};
        use wasi_websockets::{WasiWebSockets, WebSocketsDefault};

        warp::runtime!({
            main: true,
            hosts: {
                WasiHttp: HttpDefault,
                WasiOtel: OtelDefault,
                WasiWebSockets: WebSocketsDefault,
            }
        });
    } else {
        fn main() {}
    }
}
