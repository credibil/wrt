# Pub-Sub Example

This example demonstrates publishing and subscribing to messages using Apache Kafka or NATS.

The `wasi-messaging` component subscribes to topic `a.v1` and, on receipt of a message, publishes it
to topic `b.v1`.

The example uses an HTTP request to initiate publishing to topic `a.v1` as well as send 100 generated
messages to topic `b.v1`.

## Quick Start

To get started add a `.env` file to the workspace root. See `.env.example` for a template.

#### Build

```bash
# Kafka
cargo build --example pubsub --features http,messaging,kafka --target wasm32-wasip2 --release

# NATS
cargo build --example pubsub --features http,messaging,nats --target wasm32-wasip2 --release
```

#### Run

```bash
set -a && source .env && set +a

# Kafka
cargo run --features http-default,messaging,kafka -- run ./target/wasm32-wasip2/release/examples/pubsub.wasm

# NATS
cargo run --features http-default,messaging,nats -- run ./target/wasm32-wasip2/release/examples/pubsub.wasm
```

Docker Compose can also be used to run the service:

```bash
# Kafka
docker compose --file ./examples/pubsub/kafka.yaml up

# NATS
docker compose --file ./examples/pubsub/nats.yaml up
```

#### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```