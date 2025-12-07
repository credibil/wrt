#![cfg(not(target_arch = "wasm32"))]

use be_redis::Client as Redis;
use wasi_http::{WasiHttp, WasiHttpCtxImpl as HttpDefault};
use wasi_keyvalue::WasiKeyValue;
use wasi_otel::{WasiOtel, WasiOtelCtxImpl as OtelDefault};

buildgen::runtime!(main, {
    WasiHttp: HttpDefault,
    WasiOtel: OtelDefault,
    WasiKeyValue: Redis,
});
