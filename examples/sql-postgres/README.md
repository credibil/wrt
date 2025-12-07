# SQL Example (Postgres)

This example implements a simple key-value store using `wasi-sql` backed by Postgres.

An HTTP POST request will insert a (hard-coded) row into the database, while HTTP GET will query
and return all rows from a sample table.

## Quick Start

To get started add a `.env` file to the workspace root. See the `.env.example` file for a template.

### Build the WASI guest

```bash
cargo build --example sql-postgres --target wasm32-wasip2
```

### Run Postgres

Start the Postgres server and Otel Collector in a separate console:

```bash
docker compose --file ./examples/sql-postgres/postgres.yaml up
```

Run the guest:

```bash
set -a && source .env && set +a
cargo run --bin sql-postgres -- run ./target/wasm32-wasip2/debug/sql_postgres.wasm
```

### Test

```bash
# 1. INSERT
curl -X POST --header 'Content-Type: application/json' -d '{}' http://localhost:8080/

# 2. SELECT
curl http://localhost:8080/
```
