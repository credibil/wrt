#!/usr/bin/env bash

# Simple helper to load .env (if present) and run the given command.
# Usage:
#   bash scripts/env-run.sh cargo run --example http -- run ./path/to/guest.wasm

set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ -f "${REPO_ROOT}/.env" ]]; then
  # shellcheck disable=SC2046
  export $(grep -v '^#' "${REPO_ROOT}/.env" | xargs)
fi

exec "$@"
