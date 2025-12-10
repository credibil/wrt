# Vault Example (Azure)

Demonstrates `wasi-vault` backed by Azure Key Vault for secure secret storage.

## Prerequisites

- Azure Key Vault instance
- Appropriate credentials configured in `.env`

## Quick Start

```bash
# build the guest
cargo build --example vault-azure-wasm --target wasm32-wasip2

# run the host
set -a && source .env && set +a
cargo run --example vault-azure -- run ./target/wasm32-wasip2/debug/examples/vault_azure_wasm.wasm
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

## What It Does

This example demonstrates secure secret management using Azure Key Vault as the backend for `wasi-vault`.
