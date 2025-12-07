#![cfg(not(target_arch = "wasm32"))]

use wasi_http::{WasiHttp, WasiHttpCtxImpl as HttpDefault};
use wasi_otel::{WasiOtel, WasiOtelCtxImpl as OtelDefault};
use wasi_vault::{WasiVault, WasiVaultCtxImpl as VaultDefault};

buildgen::runtime!(main, {
    WasiHttp: HttpDefault,
    WasiOtel: OtelDefault,
    WasiVault: VaultDefault,
});
