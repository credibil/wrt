# Pub-Sub with Kafka Example

This demonstrates publishing and subscribing to messages using Kafka.

A simple HTTP server receives requests as a trigger via a POST request with some body that becomes a message payload. The service publishes a message on a topic, "a.v1". It also listens on that topic and publishes the message to a second topic "b.v1" (which it is listening on but does nothing with).

It will also send 100 generated messages to "b.v1".

## Quick Start

To get started add a `.env` file to the workspace root. See the `.env.example` file for a template.

In a console, build and run the `kafka` example:

```bash
# build the guest.
cargo build --example kafka --features http-default,messaging-kafka,keyvalue-redis --target wasm32-wasip2 --release

Run the services in Docker containers using docker compose.

```bash
docker compose --file ./examples/kafka/compose.yaml up
```

Use Postman or in a separate console, call the guest:

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

