# SQL Example

This example implements a simple key-value store using `wasi-sql` backed by either Postgres or
Azure Table Storage.

An HTTP POST request will insert a (hard-coded) row into the database, while HTTP GET will query
and return all rows from a sample table.

## Quick Start

To get started add a `.env` file to the workspace root. See the `.env.example` file for a template.

### Build the WASI guest

```bash
cargo build --example sql --target wasm32-wasip2 --release
```

### Run with Cargo

Start the OpenTelemetry Collector in a separate console:

```bash
docker compose --file ./examples/docker/opentelemetry.yaml up
```

Run the guest using either Postgres or Azure Table Storage:

```bash
set -a && source .env && set +a

# with Postgres
cargo run --features http,otel,sql,postgres -- run ./target/wasm32-wasip2/release/examples/sql.wasm

# with Azure Table Storage
cargo run --features http,otel,sql,azurets -- run ./target/wasm32-wasip2/release/examples/sql.wasm
```

### Run with Docker Compose

Docker Compose provides an easy way to run the example with all dependencies.

```bash
# with Postgres
docker compose --file ./examples/sql/postgres.yaml up

# with Azure Table Storage
docker compose --file ./examples/sql/azurets.yaml up
```

### Test

```bash
# 1. INSERT
curl -X POST --header 'Content-Type: application/json' -d '{}' http://localhost:8080/

# 2. SELECT
curl http://localhost:8080/
```


