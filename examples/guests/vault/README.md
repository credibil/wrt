# Vault Example

Build the example guest:

```bash
cargo build --package wasm-vault --target wasm32-wasip2 --release
```

Run the example guest:

```bash
# compile and run
cargo run --package everything -- run ./target/wasm32-wasip2/release/wasm_vault.wasm

# pre-compile
cargo run -- compile  ./target/wasm32-wasip2/release/wasm_vault.wasm --output ./wasm_vault.bin
cargo run -- run ./wasm_vault.bin
```

In a separate console, send some messages to the guest:

```bash
curl -d '{"text":"hello"}' http://localhost:8080
```
