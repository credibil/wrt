# Examples

All examples share the same shape:

1. (Optional) create `.env` at the repo root from `.env.example`.
2. Build guests: `cargo build --target wasm32-wasip2 --examples`
3. Start any required backing services with Docker Compose (see table).
4. Run the host + guest:
   `bash scripts/env-run.sh cargo run --example <example> -- run ./target/wasm32-wasip2/debug/examples/<example_with_underscores>_wasm.wasm`
   (hyphens in the example name become underscores in the built Wasm file).

To clean up services: `docker compose --file <compose-file> down -v`

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

