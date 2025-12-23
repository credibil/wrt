mod event_store;
mod types;

use crate::error::ApiError;

/// Result type used across the example
pub type Result<T> = anyhow::Result<T, ApiError>;
pub use types::*;
