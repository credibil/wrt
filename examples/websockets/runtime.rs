cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use wasi_http::{WasiHttp, WasiHttpCtxImpl as HttpDefault};
        use wasi_otel::{WasiOtel, WasiOtelCtxImpl as OtelDefault};
        use wasi_websockets::{WasiWebSockets, WasiWebSocketsCtxImpl as WebSocketsDefault};

        buildgen::runtime!(main, {
            WasiHttp: HttpDefault,
            WasiOtel: OtelDefault,
            WasiWebSockets: WebSocketsDefault,
        });
    } else {
        pub fn main() {}
    }
}
