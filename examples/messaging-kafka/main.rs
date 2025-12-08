#![cfg(not(target_arch = "wasm32"))]

use be_kafka::Client as Kafka;
use wasi_http::{WasiHttp, WasiHttpCtxImpl as HttpDefault};
use wasi_messaging::WasiMessaging;
use wasi_otel::{WasiOtel, WasiOtelCtxImpl as OtelDefault};

buildgen::runtime!(main, {
    WasiHttp: HttpDefault,
    WasiOtel: OtelDefault,
    WasiMessaging: Kafka,
});
