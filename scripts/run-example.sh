#!/usr/bin/env bash

# Helper script to build and run WASI examples
# Usage:
#   ./scripts/run-example.sh <example-name>
#   ./scripts/run-example.sh <example-name> build-only
#   ./scripts/run-example.sh <example-name> run-only

set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
EXAMPLE_NAME="${1:-}"
MODE="${2:-all}"

if [[ -z "$EXAMPLE_NAME" ]]; then
  echo "Usage: $0 <example-name> [build-only|run-only|all]"
  echo ""
  echo "Available examples:"
  echo "  blobstore, blobstore-mongodb, blobstore-nats"
  echo "  http, http-proxy"
  echo "  identity"
  echo "  keyvalue, keyvalue-nats, keyvalue-redis"
  echo "  messaging, messaging-kafka, messaging-nats"
  echo "  otel"
  echo "  sql, sql-postgres"
  echo "  vault, vault-azure"
  echo "  websockets"
  exit 1
fi

# Convert hyphenated name to underscore for wasm file
WASM_NAME="${EXAMPLE_NAME//-/_}"
WASM_FILE="${REPO_ROOT}/target/wasm32-wasip2/debug/examples/${WASM_NAME}_wasm.wasm"

# Load .env if present
if [[ -f "${REPO_ROOT}/.env" ]]; then
  echo "Loading environment from .env..."
  # shellcheck disable=SC2046
  export $(grep -v '^#' "${REPO_ROOT}/.env" | xargs)
fi

if [[ "$MODE" == "build-only" ]] || [[ "$MODE" == "all" ]]; then
  echo "Building ${EXAMPLE_NAME} guest (wasm32-wasip2)..."
  cargo build --example "${EXAMPLE_NAME}-wasm" --target wasm32-wasip2
  echo "✓ Build complete: ${WASM_FILE}"
fi

if [[ "$MODE" == "run-only" ]] || [[ "$MODE" == "all" ]]; then
  if [[ ! -f "$WASM_FILE" ]]; then
    echo "Error: WASM file not found: ${WASM_FILE}"
    echo "Run with 'build-only' or 'all' mode first."
    exit 1
  fi
  
  echo "Running ${EXAMPLE_NAME} host + guest..."
  echo "---"
  cargo run --example "${EXAMPLE_NAME}" -- run "${WASM_FILE}"
fi
