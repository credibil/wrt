cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use wasi_blobstore::{WasiBlobstore, WasiBlobstoreCtxImpl as BlobstoreDefault};
        use wasi_http::{WasiHttp, WasiHttpCtxImpl as HttpDefault};
        use wasi_otel::{WasiOtel, WasiOtelCtxImpl as OtelDefault};

        buildgen::runtime!(main, {
            WasiHttp: HttpDefault,
            WasiOtel: OtelDefault,
            WasiBlobstore: BlobstoreDefault,
        });
    } else {
        fn main() {}
    }
}
