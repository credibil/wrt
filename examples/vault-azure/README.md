# Vault Example (Azure)

This example implements a simple key-value store using `wasi-vault` backed by Azure Key Vault.

## Quick Start

Copy `.env.example` to the repo root as `.env`.

Build the guest:

```bash
cargo build --example vault-azure-wasm --target wasm32-wasip2
```

Run the host + guest:

```bash
bash scripts/env-run.sh cargo run --example vault-azure -- run ./target/wasm32-wasip2/debug/examples/vault_azure_wasm.wasm
```

Test:

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
