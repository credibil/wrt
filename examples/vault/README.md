# Key-Value Example

This example implements a simple key-value store using `wasi-vault` backed by Azure Key Vault.

## Quick Start

To get started add a `.env` file to the workspace root. See `.env.example` for a template.

#### Build

```bash
cargo build --example vault --target wasm32-wasip2 --release
```

#### Run

```bash
set -a && source .env && set +a

# Azure Key Vault
cargo run --features http,otel,vault,azure -- run ./target/wasm32-wasip2/release/examples/vault.wasm
```

Alternatively, using Docker Compose:

```bash
# Azure Key Vault
docker compose --file ./examples/vault/azurekv.yaml up
```

#### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```