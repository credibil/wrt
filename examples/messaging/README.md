# Messaging Example

This example implements a simple messaging system using `wasi-messaging` backed by either Apache
Kafka or NATS JetStream.

### Pub-Sub
The example subscribes to topic `a.v1` and, on receipt of a message, publishes it to topic `b.v1`.
An HTTP request is used to initiate publishing to topic `a.v1` as well as send 100 generated 
messages to topic `b.v1`.

### Request-Reply
The example also demonstrates a request-reply pattern where an HTTP request sends a message to topic
`req.v1` and waits for a reply on topic `rep.v1`. A separate subscriber listens on `req.v1` and
replies to each message received by publishing to `rep.v1`.

## Quick Start

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a template.

#### Build

```bash
cargo build --example messaging --target wasm32-wasip2 --release
```

#### Run

```bash
set -a && source .env && set +a

# Kafka
cargo run --features http,otel,messaging,kafka -- run ./target/wasm32-wasip2/release/examples/messaging.wasm

# NATS
cargo run --features http,otel,messaging,nats -- run ./target/wasm32-wasip2/release/examples/messaging.wasm
```

Docker Compose can also be used to run the service:

```bash
# Kafka
docker compose --file ./examples/messaging/kafka.yaml up

# NATS
docker compose --file ./examples/messaging/nats.yaml up
```

#### Test

```bash
# pub-sub
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080/pub-sub

# request-reply
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080/request-reply
```