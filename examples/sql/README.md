# SQL Example

This example implements a simple key-value store using `wasi-sql` backed by either Postgres or
Azure Table Storage.

An HTTP POST request will insert a (hard-coded) row into the database, while HTTP GET will query
and return all rows from a sample table.

## Quick Start

To get started add a `.env` file to the workspace root. See the `.env.example` file for a template.

#### Build

```bash
cargo build --example sql --target wasm32-wasip2 --release
```

#### Run

```bash
set -a && source .env && set +a

# Postgres
cargo run --features http,otel,sql,postgres -- run ./target/wasm32-wasip2/release/examples/sql.wasm

# Azure Table Storage
cargo run --features http,otel,sql,azurets -- run ./target/wasm32-wasip2/release/examples/sql.wasm
```

Docker Compose can also be used to run the service:

```bash
# Postgres
docker compose --file ./examples/sql/postgres.yaml up

# Azure Table Storage
docker compose --file ./examples/sql/azurets.yaml up
```

#### Test

```bash
# 1. INSERT
curl -X POST --header 'Content-Type: application/json' -d '{}' http://localhost:8080/

# 2. SELECT
curl http://localhost:8080/
```


