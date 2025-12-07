# Messaging Example (Kafka)

This example implements a simple messaging system using `wasi-messaging` backed by Apache Kafka.

### Pub-Sub
The example subscribes to topic `a.v1` and, on receipt of a message, publishes it to topic `b.v1`.
An HTTP request is used to initiate publishing to topic `a.v1` as well as send 100 generated
messages to topic `b.v1`.

### Request-Reply
The example also demonstrates a request-reply pattern where an HTTP request sends a message to topic
`req.v1` and waits for a reply on topic `rep.v1`. A separate subscriber listens on `req.v1` and
replies to each message received by publishing to `rep.v1`.

## Quick Start

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a
template.

### Build the WASI guest

```bash
cargo build --example messaging-kafka-wasm --target wasm32-wasip2
```

### Run Kafka

Start the Kafka broker and Otel Collector in a separate console:

```bash
docker compose --file ./examples/messaging-kafka/kafka.yaml up
```

Run the guest using:

```bash
set -a && source .env && set +a
cargo run --example messaging-kafka -- run ./target/wasm32-wasip2/debug/examples/messaging_kafka_wasm.wasm
```

### Test

```bash
# pub-sub
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080/pub-sub

# request-reply
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080/request-reply
```
