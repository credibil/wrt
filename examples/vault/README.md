# Vault Example

This example implements `wasi-vault` using the default (in memory) implementation.

## Quick Start

Copy `.env.example` to the repo root as `.env`.

Build the guest:

```bash
cargo build --example vault-wasm --target wasm32-wasip2
```

Run the host + guest:

```bash
bash scripts/env.sh cargo run --example vault -- run ./target/wasm32-wasip2/debug/examples/vault_wasm.wasm
```

Test:

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
