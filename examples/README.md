# Examples

This directory contains examples demonstrating WASI capabilities with WRT (WASI Runtime).

## Quick Start

The easiest way to run an example is using the helper script:

```bash
./scripts/run-example.sh <example-name>
```

For example:

```bash
./scripts/run-example.sh http
./scripts/run-example.sh keyvalue-redis
```

This script will:

1. Load environment variables from `.env` if present
2. Build the WebAssembly guest module
3. Run the host with the guest module loaded

### Advanced Usage

**Build only:**

```bash
./scripts/run-example.sh <example-name> build-only
```

**Run only (requires prior build):**

```bash
./scripts/run-example.sh <example-name> run-only
```

**Build all examples:**

```bash
cargo example-all
```

## Manual Workflow

If you prefer to run commands manually:

### 1. Environment Setup (Optional)

Some examples require environment variables. Copy `.env.example` to `.env` at the repo root and customize as needed.

### 2. Build the Guest (WebAssembly)

```bash
# Build a specific example
cargo build --target wasm32-wasip2 --example <example-name>-wasm

# Or build all examples
cargo build --target wasm32-wasip2 --examples
```

The compiled `.wasm` files will be in `./target/wasm32-wasip2/debug/examples/`

### 3. Start Backing Services (If Required)

Some examples require external services (see the [Catalog](#catalog) below). Start them with Docker Compose:

```bash
docker compose -f <compose-file> up -d
```

### 4. Run the Host + Guest

With environment variables:

```bash
bash scripts/env.sh cargo run --example <example-name> -- run ./target/wasm32-wasip2/debug/examples/<example_with_underscores>_wasm.wasm
```

Without environment variables:

```bash
cargo run --example <example-name> -- run ./target/wasm32-wasip2/debug/examples/<example_with_underscores>_wasm.wasm
```

**Note:** Hyphens in example names become underscores in the built Wasm filename.

### 5. Test the Example

Each example exposes an HTTP endpoint (usually `http://localhost:8080`). See individual example READMEs for specific test commands.

### 6. Clean Up

Stop and remove services:

```bash
docker compose -f <compose-file> down -v
```

## Catalog

| Example | What it demonstrates | Compose file(s) |
| --- | --- | --- |
| [blobstore](./blobstore) | `wasi-blobstore` with default backend | _none_ |
| [blobstore-mongodb](./blobstore-mongodb) | `wasi-blobstore` with MongoDB | `examples/blobstore-mongodb/mongodb.yaml` |
| [blobstore-nats](./blobstore-nats) | `wasi-blobstore` with NATS JetStream | `examples/blobstore-nats/nats.yaml` |
| [http](./http) | `wasi-http` server | `docker/otelcol.yaml` |
| [http-proxy](./http-proxy) | `wasi-http` + `wasi-keyvalue` cache | _none_ |
| [identity](./identity) | `wasi-identity` basics | _none_ |
| [keyvalue-nats](./keyvalue-nats) | `wasi-keyvalue` with NATS JetStream | `examples/keyvalue-nats/nats.yaml` |
| [keyvalue-redis](./keyvalue-redis) | `wasi-keyvalue` with Redis | `examples/keyvalue-redis/redis.yaml` |
| [messaging-kafka](./messaging-kafka) | `wasi-messaging` pub/sub + req/rep on Kafka | `examples/messaging-kafka/kafka.yaml` |
| [messaging-nats](./messaging-nats) | `wasi-messaging` pub/sub + req/rep on NATS | `examples/messaging-nats/nats.yaml` |
| [otel](./otel) | `wasi-otel` spans/metrics demo | `docker/otelcol.yaml` |
| [sql-postgres](./sql-postgres) | `wasi-sql` with Postgres | `examples/sql-postgres/postgres.yaml` |
| [vault](./vault) | `wasi-vault` basics | _none_ |
| [vault-azure](./vault-azure) | `wasi-vault` with Azure Key Vault | `examples/vault-azure/azurekv.yaml` |
| [websockets](./websockets) | `wasi-websockets` server | `docker/otelcol.yaml` |

See the linked README in each row for endpoint paths and sample curl tests.

