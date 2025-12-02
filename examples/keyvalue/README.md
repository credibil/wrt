# Key-Value Example

This example implements a simple key-value store using `wasi-keyvalue` backed by either Redis or
NATS JetStream.

## Quick Start

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a
template.

### Build the WASI guest

```bash
cargo build --example keyvalue --target wasm32-wasip2
```

### Run NATS

Start the NATS server and Otel Collector in a separate console:

```bash
docker compose --file ./examples/keyvalue/nats.yaml up
```

Run the guest:

```bash
set -a && source .env && set +a
cargo run --features http,otel,keyvalue,nats -- run ./target/wasm32-wasip2/debug/examples/keyvalue.wasm
```

### Run Redis

Start the Redis server and Otel Collector in a separate console:

```bash
docker compose --file ./examples/keyvalue/redis.yaml up
```

Run the guest using:

```bash
set -a && source .env && set +a
cargo run --features http,otel,keyvalue,redis -- run ./target/wasm32-wasip2/debug/examples/keyvalue.wasm
```


### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```