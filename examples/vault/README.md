# Key-Value Example

This example implements a simple key-value store using `wasi-vault` backed by Azure Key Vault.

## Quick Start

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a
template.

### Build the WASI guest

```bash
cargo build --example vault --target wasm32-wasip2 --release
```

### Run with Cargo

Start the OpenTelemetry Collector in a separate console:

```bash
docker compose --file ./examples/docker/opentelemetry.yaml up
```

Run the guest:

```bash
set -a && source .env && set +a
cargo run --features http,otel,vault,azure -- run ./target/wasm32-wasip2/release/examples/vault.wasm
```

Alternatively, using Docker Compose:

```bash
docker compose --file ./examples/vault/azurekv.yaml up
```

### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```