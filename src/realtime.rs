#![cfg(not(target_arch = "wasm32"))]

//! Realtime runtime CLI entry point.

use be_azure::Client as Azure;
use be_kafka::Client as Kafka;
use be_opentelemetry::Client as OpenTelemetry;
use be_redis::Client as Redis;
use wasi_http::{WasiHttp, HttpDefault};
use wasi_identity::WasiIdentity;
use wasi_keyvalue::WasiKeyValue;
use wasi_messaging::WasiMessaging;
use wasi_otel::WasiOtel;
use wasi_vault::WasiVault;

// Generate runtime
buildgen::runtime!(main, {
    WasiHttp: HttpDefault,
    WasiOtel: OpenTelemetry,
    WasiIdentity: Azure,
    WasiKeyValue: Redis,
    WasiMessaging: Kafka,
    WasiVault: Azure,
});
