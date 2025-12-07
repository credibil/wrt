# SQL Example (Postgres)

This example implements a simple key-value store using `wasi-sql` backed by Postgres.

An HTTP POST request will insert a (hard-coded) row into the database, while HTTP GET will query
and return all rows from a sample table.

## Quick Start

1. Optional: copy `.env.example` to the repo root as `.env`.
2. Build the guest:
   ```bash
   cargo build --example sql-postgres-wasm --target wasm32-wasip2
   ```
3. Start dependencies (in another terminal):
   ```bash
   docker compose --file ./examples/sql-postgres/postgres.yaml up
   ```
4. Run the host + guest:
   ```bash
   bash scripts/env-run.sh cargo run --example sql-postgres -- run ./target/wasm32-wasip2/debug/examples/sql_postgres_wasm.wasm
   ```
5. Test:
   ```bash
   # 1. INSERT
   curl -X POST --header 'Content-Type: application/json' -d '{}' http://localhost:8080/

   # 2. SELECT
   curl http://localhost:8080/
   ```
