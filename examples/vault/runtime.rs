cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use wasi_http::{WasiHttp, HttpDefault};
        use wasi_otel::{WasiOtel, OtelDefault};
        use wasi_vault::{WasiVault, VaultDefault};

        warp::runtime!({
            main: true,
            hosts: {
                WasiHttp: HttpDefault,
                WasiOtel: OtelDefault,
                WasiVault: VaultDefault,
            }
        });
    } else {
        fn main() {}
    }
}
