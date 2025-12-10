# Credibil Wasm Initiator

The Credibil Wasm runtime provides a thin wrapper around [`wasmtime`](https://github.com/bytecodealliance/wasmtime)
for ergonomic integration of host-based services for WASI components.

We consider this a stop-gap solution until production-grade runtimes support dynamic inclusion of
host-based services.

## Example Runtimes

There are a number of examples provided in the `examples` directory and a Docker compose file that
can be used to run them.

See [examples/README.md](./examples/README.md) for more details.

## Example Guests

Example guests can be found in the [examples](./examples) directory. Instructions for building and
running each example can be found in the respective README files.

## Docker Build

```bash
export CARGO_REGISTRIES_CREDIBIL_TOKEN="<registry token>"

docker build \
  --build-arg BIN="websockets" \
  --build-arg FEATURES="websockets-default" \
  --secret id=credibil,env=CARGO_REGISTRIES_CREDIBIL_TOKEN \
  --tag ghcr.io/credibil/websockets .
```
