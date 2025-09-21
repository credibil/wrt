# Blobstore Example

Build the example guest:

```bash
cargo build --package wasm-blobstore --target wasm32-wasip2 --release
```

Run the example guest:

```bash
# compile and run
cargo run --package everything -- run ./target/wasm32-wasip2/release/wasm_blobstore.wasm

# pre-compile
cargo run -- compile  ./target/wasm32-wasip2/release/wasm_blobstore.wasm --output ./wasm_blobstore.bin
cargo run -- run ./wasm_blobstore.bin
```

In a separate console, send some messages to the guest:

```bash
curl -d '{"text":"hello"}' http://localhost:8080
```
