# Vault Example (Azure)

This example implements a simple key-value store using `wasi-vault` backed by Azure Key Vault.

## Quick Start

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a
template.

### Build the WASI guest

```bash
cargo build --example vault-azure-wasm --target wasm32-wasip2
```

### Run

Start the OpenTelemetry Collector in a separate console:

```bash
docker compose --file ./docker/otelcol.yaml up
```

Run the guest:

```bash
set -a && source .env && set +a
cargo run --example vault-azure -- run ./target/wasm32-wasip2/debug/examples/vault_azure_wasm.wasm
```

### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
