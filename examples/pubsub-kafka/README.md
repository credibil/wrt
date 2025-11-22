# Pub-Sub Kafka Example

This example demonstrates publishing and subscribing to messages using Apache Kafka.

The `wasi-messaging` component subscribes to topic `a` and, on receipt of a message, publishes it
to topic `b`.

The example uses an HTTP request to initiate publishing to topic `a` as well as send 100 generated
messages to topic `b`.

## Quick Start

To get started add a `.env` file to the workspace root. See `.env.example` for a template.

#### Build

```bash
cargo build --example pubsub-kafka --features http-default,messaging-kafka --target wasm32-wasip2 --release
```

#### Run

```bash
set -a && source .env && set +a
cargo run --features http-default,messaging-kafka -- run ./target/wasm32-wasip2/release/examples/pubsub_kafka.wasm

# OR
docker compose --file ./examples/pubsub-kafka/compose.yaml up
```

#### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```