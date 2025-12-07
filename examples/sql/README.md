# SQL Example

This example implements `wasi-sql` using the default (in memory) implementation.

## Quick Start

Copy `.env.example` to the repo root as `.env`.

Build the guest:

```bash
cargo build --example sql-wasm --target wasm32-wasip2
```

Run the host + guest:

```bash
bash scripts/env-run.sh cargo run --example sql -- run ./target/wasm32-wasip2/debug/examples/sql_wasm.wasm
```

Test:

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
