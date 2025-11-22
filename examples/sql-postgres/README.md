# SQL with Postgres Example

This example demonstrates SQL queries on a Postgres server.

An HTTP POST request will insert a (hard-coded) row into the database, while HTTP GET will query
and return all rows from a sample table.

## Quick Start

To get started add a `.env` file to the workspace root. See the `.env.example` file for a template.

#### Build

```bash
cargo build --example sql-postgres --features sql-postgres --target wasm32-wasip2 --release
```

#### Run

```bash
set -a && source .env && set +a
cargo run --features sql-postgres -- run ./target/wasm32-wasip2/release/examples/sql_postgres.wasm

# OR
docker compose --file ./examples/sql-postgres/compose.yaml up
```

#### Test

```bash
# 1. INSERT
curl -X POST --header 'Content-Type: application/json' -d '{}' http://localhost:8080/

# 2. QUERY
curl http://localhost:8080/
```


