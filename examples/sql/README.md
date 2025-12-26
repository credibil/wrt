# SQL Example

Demonstrates `wasi-sql` using the default (in-memory) implementation.

## Quick Start

```bash
# build the guest
cargo build --example sql-wasm --target wasm32-wasip2

# run the host
export RUST_LOG="info,wasi_sql=debug,wasi_http=debug,sql=debug"
cargo run --example sql -- run ./target/wasm32-wasip2/debug/examples/sql_wasm.wasm
```

## Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
