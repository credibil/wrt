#![cfg(not(target_arch = "wasm32"))]

use be_kafka::Client as Kafka;
use wasi_http::{HttpDefault, WasiHttp};
use wasi_messaging::WasiMessaging;
use wasi_otel::{OtelDefault, WasiOtel};
use wasi_websockets::{WasiWebSockets, WebSocketsDefault};

buildgen::runtime!(main, {
    WasiHttp: HttpDefault,
    WasiMessaging: Kafka,
    WasiOtel: OtelDefault,
    WasiWebSockets: WebSocketsDefault,
});
