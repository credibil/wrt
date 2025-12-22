#![cfg(not(target_arch = "wasm32"))]

//! Realtime runtime builds.
//!
//! This file is used to build the realtime runtime for the different features.

// backends
#[cfg(feature = "azure")]
use be_azure::Client as Azure;
#[cfg(feature = "kafka")]
use be_kafka::Client as Kafka;
#[cfg(feature = "opentelemetry")]
use be_opentelemetry::Client as OpenTelemetry;
#[cfg(feature = "redis")]
use be_redis::Client as Redis;
// wasi components
#[cfg(feature = "http")]
use wasi_http::{HttpDefault, WasiHttp};
#[cfg(feature = "identity")]
use wasi_identity::WasiIdentity;
#[cfg(feature = "keyvalue")]
use wasi_keyvalue::WasiKeyValue;
#[cfg(feature = "messaging")]
use wasi_messaging::WasiMessaging;
#[cfg(feature = "otel")]
use wasi_otel::WasiOtel;

// Generate runtime
#[cfg(feature = "realtime")]
buildgen::runtime!(main, {
    WasiHttp: HttpDefault,
    WasiOtel: OpenTelemetry,
    WasiIdentity: Azure,
    WasiKeyValue: Redis,
    WasiMessaging: Kafka,
});

// Generate runtime
#[cfg(feature = "realtime-http")]
buildgen::runtime!(main, {
    WasiHttp: HttpDefault,
    WasiKeyValue: Redis,
    WasiOtel: OpenTelemetry,
});

// Generate runtime
#[cfg(feature = "realtime-websockets")]
buildgen::runtime!(main, {
    WasiHttp: HttpDefault,
    WasiMessaging: Kafka,
    WasiOtel: OpenTelemetry,
    WasiWebSockets: WebSocketsDefault,
});

// Generate runtime
#[cfg(not(any(feature = "realtime", feature = "realtime-http", feature = "realtime-websockets")))]
buildgen::runtime!(main, {
    WasiHttp: HttpDefault,
});
