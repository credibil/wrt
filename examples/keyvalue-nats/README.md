# Key-Value Example (NATS)

This example implements a simple key-value store using `wasi-keyvalue` backed by NATS JetStream.

## Quick Start

To get started add a `.env` file to the workspace root. See [`.env.example`](.env.example) for a
template.

### Build the WASI guest

```bash
cargo build --example keyvalue-nats --target wasm32-wasip2
```

### Run NATS

Start the NATS server and Otel Collector in a separate console:

```bash
docker compose --file ./examples/keyvalue-nats/nats.yaml up
```

Run the guest:

```bash
set -a && source .env && set +a
cargo run --bin keyvalue-nats -- run ./target/wasm32-wasip2/debug/keyvalue_nats.wasm
```

### Test

```bash
curl --header 'Content-Type: application/json' -d '{"text":"hello"}' http://localhost:8080
```
