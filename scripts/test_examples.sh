#!/usr/bin/env bash
# Test script for all examples

set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# Load .env if present
if [[ -f "${REPO_ROOT}/.env" ]]; then
  echo "Loading environment from .env..."
  # shellcheck disable=SC2046
  export $(grep -v '^#' "${REPO_ROOT}/.env" | xargs)
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track results
PASSED=()
FAILED=()
SKIPPED=()

test_example() {
    local example=$1
    local test_cmd=$2
    local needs_docker=${3:-false}
    
    echo ""
    echo "=========================================="
    echo "Testing: $example"
    echo "=========================================="
    
    # Build WASM
    echo "Building WASM..."
    if ! cargo build --example "${example}-wasm" --target wasm32-wasip2 > /tmp/${example}_build.log 2>&1; then
        echo -e "${RED}✗ Build failed${NC}"
        FAILED+=("$example (build failed)")
        return 1
    fi
    
    # Build host
    echo "Building host..."
    if ! cargo build --example "$example" > /tmp/${example}_host_build.log 2>&1; then
        echo -e "${RED}✗ Host build failed${NC}"
        FAILED+=("$example (host build failed)")
        return 1
    fi
    
    # Start Docker services if needed
    if [ "$needs_docker" = "true" ]; then
        echo "Starting Docker services..."
        # This would need to be customized per example
        echo -e "${YELLOW}⚠ Docker services needed - skipping runtime test${NC}"
        SKIPPED+=("$example (needs Docker)")
        PASSED+=("$example (build only)")
        return 0
    fi
    
    # Run server in background
    echo "Starting server..."
    local wasm_file="target/wasm32-wasip2/debug/examples/${example//-/_}_wasm.wasm"
    cargo run --example "$example" -- run "$wasm_file" > /tmp/${example}_run.log 2>&1 &
    local server_pid=$!
    
    # Give server a moment to start writing logs
    sleep 1
    
    # Wait for server to start by monitoring log output and port
    echo "Waiting for server to start..."
    local max_iterations=60  # 60 * 0.5s = 30 seconds max wait
    local iteration=0
    local server_ready=false
    
    while [ $iteration -lt $max_iterations ]; do
        # Check if process is still alive
        if ! kill -0 $server_pid 2>/dev/null; then
            echo -e "${RED}✗ Server process died before starting${NC}"
            echo "Server log:"
            cat /tmp/${example}_run.log || true
            FAILED+=("$example (server died)")
            return 1
        fi
        
        # Check for the listening message in the log or if port is listening
        if grep -q "http server listening on:" /tmp/${example}_run.log 2>/dev/null || \
           (command -v lsof >/dev/null 2>&1 && lsof -i :8080 >/dev/null 2>&1) || \
           (command -v nc >/dev/null 2>&1 && nc -z localhost 8080 >/dev/null 2>&1); then
            server_ready=true
            break
        fi
        
        sleep 0.5
        iteration=$((iteration + 1))
    done
    
    if [ "$server_ready" = "false" ]; then
        echo -e "${RED}✗ Server failed to start within 30 seconds${NC}"
        echo "Server log:"
        tail -50 /tmp/${example}_run.log || true
        kill $server_pid 2>/dev/null || true
        FAILED+=("$example (startup timeout)")
        return 1
    fi
    
    echo "Server is ready!"
    
    # Test with curl
    echo "Testing endpoint..."
    if eval "$test_cmd" > /tmp/${example}_test.log 2>&1; then
        echo -e "${GREEN}✓ Test passed${NC}"
        PASSED+=("$example")
    else
        echo -e "${RED}✗ Test failed${NC}"
        echo "Server log:"
        tail -20 /tmp/${example}_run.log || true
        echo "Test log:"
        cat /tmp/${example}_test.log || true
        FAILED+=("$example (test failed)")
    fi
    
    # Cleanup
    kill $server_pid 2>/dev/null || true
    sleep 1
    kill -9 $server_pid 2>/dev/null || true
    
    return 0
}

# Test standalone examples
test_example "blobstore" 'curl --max-time 2 --header "Content-Type: application/json" -d "{\"text\":\"hello\"}" http://localhost:8080' || true
test_example "http" 'curl --max-time 2 http://localhost:8080' || true
test_example "identity" 'curl --max-time 2 http://localhost:8080' || true
test_example "keyvalue" 'curl --max-time 2 --header "Content-Type: application/json" -d "{\"text\":\"hello\"}" http://localhost:8080' || true
test_example "messaging" 'curl --max-time 2 --header "Content-Type: application/json" -d "{\"text\":\"hello\"}" http://localhost:8080' || true
test_example "sql" 'curl --max-time 2 --header "Content-Type: application/json" -d "{\"text\":\"hello\"}" http://localhost:8080' || true
test_example "vault" 'curl --max-time 2 --header "Content-Type: application/json" -d "{\"text\":\"hello\"}" http://localhost:8080' || true

# Print summary
echo ""
echo "=========================================="
echo "SUMMARY"
echo "=========================================="
echo -e "${GREEN}Passed: ${#PASSED[@]}${NC}"
for item in "${PASSED[@]}"; do
    echo "  ✓ $item"
done

echo ""
echo -e "${YELLOW}Skipped: ${#SKIPPED[@]}${NC}"
if [ ${#SKIPPED[@]} -gt 0 ]; then
    for item in "${SKIPPED[@]}"; do
        echo "  ⚠ $item"
    done
fi

echo ""
echo -e "${RED}Failed: ${#FAILED[@]}${NC}"
if [ ${#FAILED[@]} -gt 0 ]; then
    for item in "${FAILED[@]}"; do
        echo "  ✗ $item"
    done
fi

exit ${#FAILED[@]}
