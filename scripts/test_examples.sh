#!/usr/bin/env bash
# Test script for all examples
#
# Usage:
#   ./scripts/test_examples.sh              # Test all examples
#   ./scripts/test_examples.sh http vault   # Test specific examples
#   ./scripts/test_examples.sh --help       # Show help
#
# Environment variables:
#   DEBUG=true          Enable verbose output
#   TEST_PORT=8080      Port to use (default: auto-select free port)
#   STARTUP_TIMEOUT=30  Server startup timeout in seconds
#   CURL_TIMEOUT=2      Curl request timeout in seconds

set -euo pipefail

# Enable debug mode if requested
DEBUG="${DEBUG:-false}"
[[ "$DEBUG" == "true" ]] && set -x

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# Configuration (can be overridden via environment)
STARTUP_TIMEOUT="${STARTUP_TIMEOUT:-30}"
CURL_TIMEOUT="${CURL_TIMEOUT:-2}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Track results
declare -a PASSED=()
declare -a FAILED=()
declare -a SKIPPED=()

# Temp directory for logs
LOG_DIR=""
SERVER_PIDS=()

# --- Helper Functions ---

show_help() {
    cat << EOF
Usage: $0 [OPTIONS] [example...]

Test WASI examples by building and running them.

Options:
  -h, --help     Show this help message
  -l, --list     List all available examples
  -a, --all      Test all examples (including Docker-dependent ones)

Arguments:
  example        One or more example names to test (default: all standalone examples)

Environment Variables:
  DEBUG=true          Enable verbose output (set -x)
  TEST_PORT=<port>    Port to use (default: auto-select free port)
  STARTUP_TIMEOUT=30  Server startup timeout in seconds
  CURL_TIMEOUT=2      Curl request timeout in seconds

Examples:
  $0                    # Test all standalone examples
  $0 http keyvalue      # Test only http and keyvalue examples
  DEBUG=true $0 http    # Test http with debug output

Available Examples:
  Standalone (tested by default):
    blobstore      - blob storage with in-memory backend
    http           - HTTP request handling
    http-proxy     - HTTP proxy with caching (tests /echo endpoint)
    identity       - identity/auth (needs IDENTITY_TOKEN_URL - skipped if missing)
    keyvalue       - key-value store with in-memory backend
    messaging      - pub-sub messaging (build-only, needs broker for runtime)
    otel           - OpenTelemetry instrumentation
    sql            - SQL database (tests query endpoint)
    vault          - secrets vault with in-memory backend
    websockets     - WebSocket server (tests /health endpoint)

  Docker-dependent (auto-skipped if services not running):
    blobstore-mongodb  - docker compose -f docker/mongodb.yaml up -d
    blobstore-nats     - docker compose -f docker/nats.yaml up -d
    keyvalue-nats      - docker compose -f docker/nats.yaml up -d
    keyvalue-redis     - docker compose -f docker/redis.yaml up -d
    messaging-kafka    - docker compose -f docker/kafka.yaml up -d
    messaging-nats     - docker compose -f docker/nats.yaml up -d
    sql-postgres       - docker compose -f docker/postgres.yaml up -d
    vault-azure        - docker compose -f docker/azurekv.yaml up -d
EOF
}

list_examples() {
    echo "Available examples:"
    echo ""
    echo "Standalone (tested by default):"
    echo "  blobstore      - blob storage with in-memory backend"
    echo "  http           - HTTP request handling"
    echo "  http-proxy     - HTTP proxy with caching (tests /echo endpoint)"
    echo "  identity       - identity/auth (needs IDENTITY_TOKEN_URL - skipped if missing)"
    echo "  keyvalue       - key-value store with in-memory backend"
    echo "  messaging      - pub-sub messaging (build-only, needs broker for runtime)"
    echo "  otel           - OpenTelemetry instrumentation"
    echo "  sql            - SQL database (tests query endpoint)"
    echo "  vault          - secrets vault with in-memory backend"
    echo "  websockets     - WebSocket server (tests /health endpoint)"
    echo ""
    echo "Docker-dependent (auto-skipped if services not running):"
    echo "  blobstore-mongodb  - docker compose -f docker/mongodb.yaml up -d"
    echo "  blobstore-nats     - docker compose -f docker/nats.yaml up -d"
    echo "  keyvalue-nats      - docker compose -f docker/nats.yaml up -d"
    echo "  keyvalue-redis     - docker compose -f docker/redis.yaml up -d"
    echo "  messaging-kafka    - docker compose -f docker/kafka.yaml up -d"
    echo "  messaging-nats     - docker compose -f docker/nats.yaml up -d"
    echo "  sql-postgres       - docker compose -f docker/postgres.yaml up -d"
    echo "  vault-azure        - docker compose -f docker/azurekv.yaml up -d"
}

cleanup() {
    echo ""
    echo "Cleaning up..."
    
    # Kill any server processes we started
    if [[ ${#SERVER_PIDS[@]} -gt 0 ]]; then
        for pid in "${SERVER_PIDS[@]}"; do
            if kill -0 "$pid" 2>/dev/null; then
                kill "$pid" 2>/dev/null || true
                sleep 0.5
                kill -9 "$pid" 2>/dev/null || true
            fi
        done
    fi
    
    # Also kill any remaining background jobs
    local bg_pids
    bg_pids=$(jobs -p 2>/dev/null) || true
    if [[ -n "$bg_pids" ]]; then
        echo "$bg_pids" | xargs kill 2>/dev/null || true
    fi
    
    # Clean up temp directory
    if [[ -n "$LOG_DIR" && -d "$LOG_DIR" ]]; then
        if [[ "$DEBUG" == "true" ]]; then
            echo "Preserving logs in: $LOG_DIR"
        else
            rm -rf "$LOG_DIR"
        fi
    fi
}

trap cleanup EXIT INT TERM

get_free_port() {
    python3 -c 'import socket; s=socket.socket(); s.bind(("", 0)); print(s.getsockname()[1]); s.close()' 2>/dev/null \
        || python -c 'import socket; s=socket.socket(); s.bind(("", 0)); print(s.getsockname()[1]); s.close()' 2>/dev/null \
        || echo "8080"
}

docker_service_running() {
    local compose_file=$1
    if [[ ! -f "$compose_file" ]]; then
        return 1
    fi
    
    # Check if docker compose services are running
    if command -v docker >/dev/null 2>&1; then
        docker compose -f "$compose_file" ps --status running 2>/dev/null | grep -qv "NAME" && return 0
    fi
    return 1
}

wait_for_server() {
    local pid=$1
    local port=$2
    local log_file=$3
    local max_iterations=$((STARTUP_TIMEOUT * 2))  # Check every 0.5s
    local iteration=0
    
    while [ $iteration -lt $max_iterations ]; do
        # Check if process is still alive
        if ! kill -0 "$pid" 2>/dev/null; then
            return 1
        fi
        
        # Check for the listening message in the log or if port is listening
        if grep -q "http server listening on:" "$log_file" 2>/dev/null || \
           grep -q "listening on" "$log_file" 2>/dev/null || \
           (command -v lsof >/dev/null 2>&1 && lsof -i ":$port" >/dev/null 2>&1) || \
           (command -v nc >/dev/null 2>&1 && nc -z localhost "$port" >/dev/null 2>&1); then
            return 0
        fi
        
        sleep 0.5
        iteration=$((iteration + 1))
    done
    
    return 1
}

# --- Test Functions ---

# Test configuration for each example
# Format: needs_docker|docker_compose_file|method|path|data|required_env_vars
#
# Examples with specific routes:
#   http-proxy: /echo (GET), /cache (GET), /origin (POST)
#   websockets: /health (GET), /socket (POST)
#   messaging: /pub-sub (POST), /request-reply (POST)
#   sql: / (GET=query, POST=insert)
#   identity: / (GET)
#
get_example_config() {
    local example=$1
    case "$example" in
        # Standalone examples - no Docker needed
        blobstore)          echo "false||POST|/|{\"text\":\"hello\"}|" ;;
        http)               echo "false||POST|/|{\"text\":\"hello\"}|" ;;
        keyvalue)           echo "false||POST|/|{\"text\":\"hello\"}|" ;;
        otel)               echo "false||POST|/|{\"text\":\"hello\"}|" ;;
        vault)              echo "false||POST|/|{\"text\":\"hello\"}|" ;;
        
        # Standalone examples - need specific routes or env vars
        http-proxy)         echo "false||GET|/cache||" ;;  # /cache fetches and caches external URL
        identity)           echo "false||GET|/||IDENTITY_TOKEN_URL" ;;
        websockets)         echo "false||GET|/health||" ;;
        
        # Messaging needs a broker connection - skip HTTP test, just build
        messaging)          echo "false||BUILD_ONLY|||" ;;
        
        # SQL needs database connection - skip HTTP test, just build
        sql)                echo "false||BUILD_ONLY|||" ;;
        
        # Docker-dependent examples
        blobstore-mongodb)  echo "true|docker/mongodb.yaml|POST|/|{\"text\":\"hello\"}|" ;;
        blobstore-nats)     echo "true|docker/nats.yaml|POST|/|{\"text\":\"hello\"}|" ;;
        keyvalue-nats)      echo "true|docker/nats.yaml|POST|/|{\"text\":\"hello\"}|" ;;
        keyvalue-redis)     echo "true|docker/redis.yaml|POST|/|{\"text\":\"hello\"}|" ;;
        messaging-kafka)    echo "true|docker/kafka.yaml|POST|/pub-sub|{\"text\":\"hello\"}|" ;;
        messaging-nats)     echo "true|docker/nats.yaml|POST|/request-reply|{\"text\":\"hello\"}|" ;;
        sql-postgres)       echo "true|docker/postgres.yaml|GET|/||" ;;
        vault-azure)        echo "true|docker/azurekv.yaml|POST|/|{\"text\":\"hello\"}|" ;;
        
        *)                  echo "unknown" ;;
    esac
}

# Check if required environment variables are set
check_required_env() {
    local required_vars=$1
    local missing_vars=()
    
    if [[ -z "$required_vars" ]]; then
        return 0
    fi
    
    IFS=',' read -ra vars <<< "$required_vars"
    for var in "${vars[@]}"; do
        if [[ -z "${!var:-}" ]]; then
            missing_vars+=("$var")
        fi
    done
    
    if [[ ${#missing_vars[@]} -gt 0 ]]; then
        echo "${missing_vars[*]}"
        return 1
    fi
    return 0
}

run_curl_test() {
    local port=$1
    local method=$2
    local path=$3
    local data=$4
    local log_file=$5
    
    local curl_args=(
        --max-time "$CURL_TIMEOUT"
        --silent
        --show-error
        --fail
        -o "$log_file"
        -w "%{http_code}"
    )
    
    if [[ "$method" == "POST" && -n "$data" ]]; then
        curl_args+=(--header "Content-Type: application/json")
        curl_args+=(--data "$data")
    elif [[ "$method" == "GET" && -n "$data" ]]; then
        # For GET requests with data, send as JSON body (some APIs support this)
        curl_args+=(--header "Content-Type: application/json")
        curl_args+=(--data "$data")
    fi
    
    curl_args+=("http://localhost:${port}${path}")
    
    local http_code
    http_code=$(curl "${curl_args[@]}" 2>>"$log_file") || return 1
    
    # Check for successful HTTP codes (2xx)
    if [[ "$http_code" =~ ^2[0-9][0-9]$ ]]; then
        return 0
    else
        echo "HTTP $http_code" >> "$log_file"
        return 1
    fi
}

test_example() {
    local example=$1
    local start_time
    start_time=$(date +%s)
    
    echo ""
    echo "=========================================="
    echo -e "${BLUE}Testing: $example${NC}"
    echo "=========================================="
    
    # Get configuration for this example
    local config
    config=$(get_example_config "$example")
    
    if [[ "$config" == "unknown" ]]; then
        echo -e "${YELLOW}⚠ Unknown example: $example${NC}"
        SKIPPED+=("$example (unknown)")
        return 0
    fi
    
    IFS='|' read -r needs_docker docker_compose test_method test_path test_data required_env <<< "$config"
    
    # Check required environment variables
    if [[ -n "$required_env" ]]; then
        local missing_vars
        if ! missing_vars=$(check_required_env "$required_env"); then
            echo -e "${YELLOW}⚠ Missing required environment variables: $missing_vars${NC}"
            echo "  Set these in .env or export them before running"
            
            # Still try to build
            echo "Building WASM (build-only test)..."
            if cargo build --example "${example}-wasm" --target wasm32-wasip2 > "$LOG_DIR/${example}_build.log" 2>&1; then
                echo -e "${GREEN}✓ Build succeeded${NC}"
                PASSED+=("$example (build only)")
            else
                echo -e "${RED}✗ Build failed${NC}"
                [[ "$DEBUG" == "true" ]] && cat "$LOG_DIR/${example}_build.log"
                FAILED+=("$example (build failed)")
            fi
            SKIPPED+=("$example (missing env: $missing_vars)")
            print_duration "$start_time"
            return 0
        fi
    fi
    
    # Check Docker dependencies
    if [[ "$needs_docker" == "true" ]]; then
        if [[ -n "$docker_compose" ]] && docker_service_running "$REPO_ROOT/$docker_compose"; then
            echo "Docker services detected as running"
        else
            echo -e "${YELLOW}⚠ Docker services needed but not running${NC}"
            echo "  Start with: docker compose -f $docker_compose up -d"
            
            # Still try to build
            echo "Building WASM (build-only test)..."
            if cargo build --example "${example}-wasm" --target wasm32-wasip2 > "$LOG_DIR/${example}_build.log" 2>&1; then
                echo -e "${GREEN}✓ Build succeeded${NC}"
                PASSED+=("$example (build only)")
            else
                echo -e "${RED}✗ Build failed${NC}"
                [[ "$DEBUG" == "true" ]] && cat "$LOG_DIR/${example}_build.log"
                FAILED+=("$example (build failed)")
            fi
            SKIPPED+=("$example (needs Docker)")
            print_duration "$start_time"
            return 0
        fi
    fi
    
    # Build WASM
    echo "Building WASM..."
    if ! cargo build --example "${example}-wasm" --target wasm32-wasip2 > "$LOG_DIR/${example}_build.log" 2>&1; then
        echo -e "${RED}✗ Build failed${NC}"
        [[ "$DEBUG" == "true" ]] && cat "$LOG_DIR/${example}_build.log"
        FAILED+=("$example (build failed)")
        print_duration "$start_time"
        return 1
    fi
    
    # Build host
    echo "Building host..."
    if ! cargo build --example "$example" > "$LOG_DIR/${example}_host_build.log" 2>&1; then
        echo -e "${RED}✗ Host build failed${NC}"
        [[ "$DEBUG" == "true" ]] && cat "$LOG_DIR/${example}_host_build.log"
        FAILED+=("$example (host build failed)")
        print_duration "$start_time"
        return 1
    fi
    
    # Handle BUILD_ONLY mode (for examples that can't be HTTP tested)
    if [[ "$test_method" == "BUILD_ONLY" ]]; then
        echo -e "${GREEN}✓ Build succeeded (runtime test skipped)${NC}"
        PASSED+=("$example (build only)")
        print_duration "$start_time"
        return 0
    fi
    
    # Get a free port for this test
    local port
    port="${TEST_PORT:-$(get_free_port)}"
    echo "Using port: $port"
    
    # Run server in background
    echo "Starting server..."
    local wasm_file="target/wasm32-wasip2/debug/examples/${example//-/_}_wasm.wasm"
    
    # Set HTTP_ADDR environment variable for the server (used by wasi-http)
    HTTP_ADDR="0.0.0.0:$port" cargo run --example "$example" -- run "$wasm_file" > "$LOG_DIR/${example}_run.log" 2>&1 &
    local server_pid=$!
    SERVER_PIDS+=("$server_pid")
    
    # Give server a moment to start writing logs
    sleep 1
    
    # Wait for server to start
    echo "Waiting for server to start (timeout: ${STARTUP_TIMEOUT}s)..."
    if ! wait_for_server "$server_pid" "$port" "$LOG_DIR/${example}_run.log"; then
        if ! kill -0 "$server_pid" 2>/dev/null; then
            echo -e "${RED}✗ Server process died before starting${NC}"
            echo "Server log:"
            cat "$LOG_DIR/${example}_run.log" || true
            FAILED+=("$example (server died)")
        else
            echo -e "${RED}✗ Server failed to start within ${STARTUP_TIMEOUT} seconds${NC}"
            echo "Server log:"
            tail -50 "$LOG_DIR/${example}_run.log" || true
            kill "$server_pid" 2>/dev/null || true
            FAILED+=("$example (startup timeout)")
        fi
        print_duration "$start_time"
        return 1
    fi
    
    echo "Server is ready!"
    
    # Run the test
    echo "Testing endpoint ($test_method $test_path)..."
    if run_curl_test "$port" "$test_method" "$test_path" "$test_data" "$LOG_DIR/${example}_test.log"; then
        echo -e "${GREEN}✓ Test passed${NC}"
        PASSED+=("$example")
    else
        echo -e "${RED}✗ Test failed${NC}"
        echo "Server log (last 20 lines):"
        tail -20 "$LOG_DIR/${example}_run.log" || true
        echo "Test log:"
        cat "$LOG_DIR/${example}_test.log" || true
        FAILED+=("$example (test failed)")
    fi
    
    # Cleanup server
    kill "$server_pid" 2>/dev/null || true
    sleep 0.5
    kill -9 "$server_pid" 2>/dev/null || true
    
    print_duration "$start_time"
    return 0
}

print_duration() {
    local start_time=$1
    local duration=$(($(date +%s) - start_time))
    echo -e "  ${BLUE}Duration: ${duration}s${NC}"
}

print_summary() {
    echo ""
    echo "=========================================="
    echo "SUMMARY"
    echo "=========================================="
    
    echo -e "${GREEN}Passed: ${#PASSED[@]}${NC}"
    if [[ ${#PASSED[@]} -gt 0 ]]; then
        for item in "${PASSED[@]}"; do
            echo "  ✓ $item"
        done
    fi
    
    echo ""
    echo -e "${YELLOW}Skipped: ${#SKIPPED[@]}${NC}"
    if [[ ${#SKIPPED[@]} -gt 0 ]]; then
        for item in "${SKIPPED[@]}"; do
            echo "  ⚠ $item"
        done
    fi
    
    echo ""
    echo -e "${RED}Failed: ${#FAILED[@]}${NC}"
    if [[ ${#FAILED[@]} -gt 0 ]]; then
        for item in "${FAILED[@]}"; do
            echo "  ✗ $item"
        done
    fi
    
    if [[ "$DEBUG" == "true" && -n "$LOG_DIR" ]]; then
        echo ""
        echo "Logs preserved in: $LOG_DIR"
    fi
}

# --- Main ---

main() {
    # Parse arguments
    local examples_to_test=()
    local test_all=false
    
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -h|--help)
                show_help
                exit 0
                ;;
            -l|--list)
                list_examples
                exit 0
                ;;
            -a|--all)
                test_all=true
                ;;
            -*)
                echo "Unknown option: $1"
                echo "Use --help for usage information"
                exit 1
                ;;
            *)
                examples_to_test+=("$1")
                ;;
        esac
        shift
    done
    
    # Load .env if present
    if [[ -f "${REPO_ROOT}/.env" ]]; then
        echo "Loading environment from .env..."
        while IFS='=' read -r key value; do
            # Skip empty lines and comments
            [[ -z "$key" || "$key" =~ ^[[:space:]]*# ]] && continue
            # Remove surrounding quotes from value if present
            value="${value%\"}"
            value="${value#\"}"
            value="${value%\'}"
            value="${value#\'}"
            # Export the variable
            export "$key=$value"
        done < "${REPO_ROOT}/.env"
    fi
    
    # Create temp directory for logs
    LOG_DIR=$(mktemp -d)
    echo "Log directory: $LOG_DIR"
    
    # Determine which examples to test
    if [[ ${#examples_to_test[@]} -eq 0 ]]; then
        if [[ "$test_all" == "true" ]]; then
            # All examples including Docker-dependent ones
            examples_to_test=(
                blobstore
                blobstore-mongodb
                blobstore-nats
                http
                http-proxy
                identity
                keyvalue
                keyvalue-nats
                keyvalue-redis
                messaging
                messaging-kafka
                messaging-nats
                otel
                sql
                sql-postgres
                vault
                vault-azure
                websockets
            )
        else
            # Default: standalone examples only
            examples_to_test=(
                blobstore
                http
                http-proxy
                identity
                keyvalue
                messaging
                otel
                sql
                vault
                websockets
            )
        fi
    fi
    
    echo ""
    echo "Testing ${#examples_to_test[@]} example(s): ${examples_to_test[*]}"
    
    # Run tests
    for example in "${examples_to_test[@]}"; do
        test_example "$example" || true
    done
    
    # Print summary
    print_summary
    
    # Exit with appropriate code (capped at 125)
    local exit_code=${#FAILED[@]}
    exit $((exit_code > 125 ? 125 : exit_code))
}

main "$@"
