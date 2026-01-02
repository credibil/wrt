cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use wasi_blobstore::{WasiBlobstore, BlobstoreDefault};
        use wasi_http::{WasiHttp, HttpDefault};
        use wasi_otel::{WasiOtel, OtelDefault};

        warp::runtime!({
            main: true,
            hosts: {
                WasiHttp: HttpDefault,
                WasiOtel: OtelDefault,
                WasiBlobstore: BlobstoreDefault,
            }
        });
    } else {
        fn main() {}
    }
}
