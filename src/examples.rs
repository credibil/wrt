#![cfg(not(target_arch = "wasm32"))]
// #![cfg(all(feature = "http", feature = "identity", feature = "otel"))]

//! WRT runtime CLI entry point.

use anyhow::Result;
use kernel::{Cli, Command, Parser};
use wasi_blobstore::{WasiBlobstore, WasiBlobstoreCtxImpl as BlobstoreDefault};
use wasi_http::{WasiHttp, WasiHttpCtxImpl as HttpDefault};
use wasi_identity::{WasiIdentity, WasiIdentityCtxImpl as IdentityDefault};
use wasi_keyvalue::{WasiKeyValue, WasiKeyValueCtxImpl as KeyValueDefault};
use wasi_messaging::{WasiMessaging, WasiMessagingCtxImpl as MessagingDefault};
use wasi_otel::{WasiOtel, WasiOtelCtxImpl as OtelDefault};
use wasi_sql::{WasiSql, WasiSqlCtxImpl as SqlDefault};
use wasi_vault::{WasiVault, WasiVaultCtxImpl as VaultDefault};
use wasi_websockets::{WasiWebSockets, WasiWebSocketsCtxImpl as WebSocketsDefault};

// Generate runtime infrastructure for the credibil feature set
buildgen::runtime!({
    WasiBlobstore: BlobstoreDefault,
    WasiHttp: HttpDefault,
    WasiIdentity: IdentityDefault,
    WasiKeyValue: KeyValueDefault,
    WasiMessaging: MessagingDefault,
     WasiOtel: OtelDefault,
    WasiSql: SqlDefault,
    WasiVault: VaultDefault,
    WasiWebSockets: WebSocketsDefault,

});

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Run { wasm } => runtime::run(wasm).await,
        _ => unreachable!(),
    }
}
