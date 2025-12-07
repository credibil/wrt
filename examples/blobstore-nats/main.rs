#![cfg(not(target_arch = "wasm32"))]

use be_nats::Client as Nats;
use wasi_blobstore::WasiBlobstore;
use wasi_http::{WasiHttp, WasiHttpCtxImpl as HttpDefault};
use wasi_otel::{WasiOtel, WasiOtelCtxImpl as OtelDefault};

buildgen::runtime!(main, {
    WasiHttp: HttpDefault,
    WasiOtel: OtelDefault,
    WasiBlobstore: Nats,
});
