# Pub-Sub with NATS Example

This demonstrates publishing and subscribing to messages using NATS.

A simple HTTP server receives requests as a trigger via a POST request with some body that becomes a message payload. The service publishes a message on a topic, "a". It also listens on that topic and publishes the message to a second topic "b" (which it is listening on but does nothing with).

It will also send 100 generated messages to "b".

## Quick Start

To get started add a `.env` file to the workspace root. See the `.env.example` file for a template.

In a console, build and run the `pub-sub` example:

```bash
# build the guest.
cargo build --example pub-sub --features http-default,messaging-nats,keyvalue-nats --target wasm32-wasip2 --release

Run the services in Docker containers using docker compose.

```bash
docker compose --file ./examples/pub_sub/compose.yaml up
```

Use Postman or in a separate console, call the guest:

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```

