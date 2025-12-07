#![cfg(not(target_arch = "wasm32"))]

use be_nats::Client as Nats;
use wasi_http::{WasiHttp, WasiHttpCtxImpl as HttpDefault};
use wasi_keyvalue::WasiKeyValue;
use wasi_otel::{WasiOtel, WasiOtelCtxImpl as OtelDefault};

buildgen::runtime!(main, {
    WasiHttp: HttpDefault,
    WasiOtel: OtelDefault,
    WasiKeyValue: Nats,
});
