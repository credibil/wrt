# Default Backend Implementations

This document describes the lightweight default backend implementations created for each `wasi-*` crate. These implementations are intended for developer use only and should not be used in production.

## Overview

Default implementations have been created for the following crates:

1. **wasi-keyvalue** - HashMap-based key-value store
2. **wasi-blobstore** - HashMap-based blob storage
3. **wasi-vault** - HashMap-based secret storage
4. **wasi-messaging** - In-memory message queue
5. **wasi-sql** - SQLite-based SQL database

## Implementation Details

### wasi-keyvalue (`crates/wasi-keyvalue/src/host/default_impl.rs`)

- **Backend**: In-memory HashMap with `parking_lot::RwLock` for thread-safety
- **Features**:
  - Multiple buckets support
  - Basic CRUD operations (get, set, delete, exists, keys)
  - Thread-safe concurrent access
- **Usage**: No configuration required (empty `ConnectOptions`)

### wasi-blobstore (`crates/wasi-blobstore/src/host/default_impl.rs`)

- **Backend**: In-memory HashMap for containers and objects
- **Features**:
  - Container management (create, get, delete, exists)
  - Object operations (read, write, list, delete, has, info)
  - Metadata tracking (creation time, size)
- **Limitations**: Range reads are not implemented (reads entire object)
- **Usage**: No configuration required

### wasi-vault (`crates/wasi-vault/src/host/default_impl.rs`)

- **Backend**: In-memory HashMap with `parking_lot::RwLock`
- **Features**:
  - Multiple locker support
  - Secret management (get, set, delete, exists, list)
  - Thread-safe concurrent access
- **Usage**: No configuration required

### wasi-messaging (`crates/wasi-messaging/src/host/default_impl.rs`)

- **Backend**: In-memory HashMap for message storage
- **Features**:
  - Message creation and manipulation
  - Topic-based send operations
  - Request-reply pattern (returns simple ACK)
  - Metadata management
- **Limitations**: 
  - Subscribe returns empty stream
  - No actual message delivery
  - Request always returns simple ACK response
- **Usage**: No configuration required

### wasi-sql (`crates/wasi-sql/src/host/default_impl.rs`)

- **Backend**: SQLite with `rusqlite` library
- **Features**:
  - Full SQL support via SQLite
  - Query and execute operations
  - Parameterized queries
  - Type conversion between WASI types and SQLite types
- **Configuration**:
  - `SQL_DATABASE` environment variable (defaults to `:memory:`)
  - Supports file-based or in-memory databases
- **Usage**: Set `SQL_DATABASE=/path/to/db.sqlite` or use default in-memory database

## Usage Example

To use a default implementation:

```rust
use wasi_keyvalue::host::default_impl::WasiKeyValueCtxImpl;
use kernel::Backend;

// Connect using default options (from environment or defaults)
let ctx = WasiKeyValueCtxImpl::connect().await?;

// Or with explicit options
let ctx = WasiKeyValueCtxImpl::connect_with(ConnectOptions::default()).await?;
```

## Dependencies Added

The following dependencies were added to support the default implementations:

- `parking_lot = "0.12"` - For efficient RwLock implementations
- `fromenv` (workspace) - For environment configuration (not needed for unit struct ConnectOptions)
- `rusqlite = { version = "0.32", features = ["bundled"] }` - For SQLite support (wasi-sql only)
- `tokio` (dev-dependencies) - For tests

## Testing

Each implementation includes basic unit tests demonstrating the functionality. Run tests with:

```bash
cargo test --package wasi-keyvalue
cargo test --package wasi-blobstore
cargo test --package wasi-vault
cargo test --package wasi-messaging
cargo test --package wasi-sql
```

## Existing Implementations

The following crates already had default implementations:

- **wasi-http** - Uses `reqwest` for HTTP client functionality
- **wasi-identity** - OAuth2 client credentials flow
- **wasi-otel** - gRPC-based OpenTelemetry exporter
- **wasi-websockets** - Default WebSocket server implementation

## Production Use

⚠️ **Warning**: These default implementations are for development and testing only. For production use, consider:

- **wasi-keyvalue**: Use `be-redis` or `be-nats`
- **wasi-blobstore**: Use `be-mongodb` or `be-nats`
- **wasi-vault**: Use `be-azure` (Key Vault) or proper secret management
- **wasi-messaging**: Use `be-kafka` or `be-nats`
- **wasi-sql**: Use `be-postgres` or other production databases

## Notes

- All in-memory implementations will lose data when the process terminates
- Thread-safety is provided via `parking_lot::RwLock` where needed
- The implementations prioritize simplicity over performance
- SQLite is the only persistent option among the defaults (when using file-based database)

