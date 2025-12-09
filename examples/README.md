# Examples

This directory contains examples demonstrating WASI capabilities with WRT (WASI Runtime).

## Understanding the Architecture

Each example follows a **guest/host** architecture:

```
┌─────────────────────────────────────────────────────────┐
│                        Host                             │
│  (Native binary that provides WASI implementations)     │
│                                                         │
│   ┌─────────────────────────────────────────────────┐   │
│   │                    Guest                        │   │
│   │  (WebAssembly module with your business logic)  │   │
│   │                                                 │   │
│   │   lib.rs  ──────►  .wasm file                   │   │
│   └─────────────────────────────────────────────────┘   │
│                                                         │
│   main.rs  ──────►  native binary                       │
└─────────────────────────────────────────────────────────┘
```

- **Guest (`lib.rs`)**: Application code compiled to WebAssembly. It uses WASI interfaces to interact with the outside world.
- **Host (`main.rs`)**: The runtime that loads and executes the Wasm guest, providing concrete implementations of WASI interfaces (e.g., connecting to backends such as Redis, Kafka, Postgres).

This separation allows the same guest code to run with different backends—swap Redis for NATS without changing your application logic.

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

## Learning Path

Examples are organized by complexity. Start with **Beginner** examples to understand the fundamentals before moving to more advanced patterns.

### Beginner

These examples demonstrate single WASI interfaces with minimal setup (no external services required).

| Example | What it demonstrates | Key concepts |
| --- | --- | --- |
| [http](./http) | Basic HTTP server | WASI HTTP handler, Axum routing, JSON responses |
| [keyvalue](./keyvalue) | In-memory key-value storage | WASI KeyValue store API |
| [blobstore](./blobstore) | In-memory blob storage | WASI Blobstore API |
| [vault](./vault) | In-memory secrets vault | WASI Vault API |
| [identity](./identity) | Identity/auth basics | WASI Identity API |

### Intermediate

These examples add observability, multiple endpoints, or external service backends.

| Example | What it demonstrates | Compose file |
| --- | --- | --- |
| [otel](./otel) | Distributed tracing & metrics | `docker/otelcol.yaml` |
| [websockets](./websockets) | WebSocket server | `docker/otelcol.yaml` |
| [http-proxy](./http-proxy) | HTTP proxy with caching | _none_ |
| [keyvalue-redis](./keyvalue-redis) | KeyValue with Redis backend | `examples/keyvalue-redis/redis.yaml` |
| [keyvalue-nats](./keyvalue-nats) | KeyValue with NATS backend | `examples/keyvalue-nats/nats.yaml` |
| [blobstore-mongodb](./blobstore-mongodb) | Blobstore with MongoDB | `examples/blobstore-mongodb/mongodb.yaml` |
| [blobstore-nats](./blobstore-nats) | Blobstore with NATS | `examples/blobstore-nats/nats.yaml` |

### Advanced

These examples demonstrate complex patterns like pub/sub messaging, database operations, and cloud integrations.

| Example | What it demonstrates | Compose file |
| --- | --- | --- |
| [sql-postgres](./sql-postgres) | SQL queries with Postgres | `examples/sql-postgres/postgres.yaml` |
| [messaging-kafka](./messaging-kafka) | Pub/sub with Kafka (1000 msg fan-out) | `examples/messaging-kafka/kafka.yaml` |
| [messaging-nats](./messaging-nats) | Pub/sub with NATS | `examples/messaging-nats/nats.yaml` |
| [vault-azure](./vault-azure) | Secrets with Azure Key Vault | `examples/vault-azure/azurekv.yaml` |

## Troubleshooting

### Common Issues

**Build fails with "target not found"**
```bash
rustup target add wasm32-wasip2
```

**Server won't start / port already in use**
```bash
# Find and kill the process using port 8080
lsof -i :8080 | awk 'NR>1 {print $2}' | xargs kill
```

**Docker services not connecting**
```bash
# Check if services are running
docker compose -f <compose-file> ps

# View service logs
docker compose -f <compose-file> logs
```

**WASM module panics at runtime**
- Enable debug logging: `RUST_LOG=debug cargo run --example <name> -- run <wasm>`
- Check that required environment variables are set (see `.env.example`)

See the linked README in each example for specific endpoint paths and test commands.

