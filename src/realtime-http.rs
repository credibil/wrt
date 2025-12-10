#![cfg(not(target_arch = "wasm32"))]

//! Realtime runtime CLI entry point.

use be_opentelemetry::Client as OpenTelemetry;
use be_redis::Client as Redis;
use wasi_http::{WasiHttp, WasiHttpCtxImpl};
use wasi_keyvalue::WasiKeyValue;
use wasi_otel::WasiOtel;

// Generate runtime
buildgen::runtime!(main, {
    WasiHttp: WasiHttpCtxImpl,
    WasiOtel: OpenTelemetry,
    WasiKeyValue: Redis,
});
