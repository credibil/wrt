# SQL Example

Demonstrates `wasi-sql` using the Postgres implementation.

## Quick Start

```bash
# build the guest
cargo build --example sql-pg-wasm --target wasm32-wasip2

# run the host
set -a && source .env && set +a
cargo run --example sql-pg --features postgres -- run ./target/wasm32-wasip2/debug/examples/sql_pg_wasm.wasm

```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
