use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tracing::{error, info, warn};

#[derive(Clone, Debug, Deserialize, Eq, Error, PartialEq, Serialize)]
pub enum Error {
    #[error("{{\"code\": 503, \"description\": \"{0}\"}}")]
    ApplicationError(String),

    #[error("{{\"code\": 502, \"description\": \"{0}\"}}")]
    ExternalError(String),

    #[error("{{\"code\": 400, \"description\": \"{0}\"}}")]
    InvalidInput(String),

    #[error("{{\"code\": 404, \"description\": \"{0}\"}}")]
    NotFound(String),

    #[error("{{\"code\": 520, \"description\": \"{0}\"}}")]
    Other(String),

    #[error("{{\"code\": 410, \"description\": \"{0}\"}}")]
    Outdated(String),

    #[error("{{\"code\": 500, \"description\": \"{0}\"}}")]
    ServerError(String),
}

impl Error {
    /// Returns the error code.
    #[must_use]
    pub const fn code(&self) -> u64 {
        match self {
            Self::ApplicationError(_) => 503,
            Self::ExternalError(_) => 502,
            Self::InvalidInput(_) => 400,
            Self::NotFound(_) => 404,
            Self::Other(_) => 520,
            Self::Outdated(_) => 410,
            Self::ServerError(_) => 500,
        }
    }

    /// Returns the error description.
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            Self::ApplicationError(desc)
            | Self::ExternalError(desc)
            | Self::InvalidInput(desc)
            | Self::NotFound(desc)
            | Self::Other(desc)
            | Self::Outdated(desc)
            | Self::ServerError(desc) => desc.clone(),
        }
    }

    /// Performs tracing and metrics.
    pub fn trace(&self, service: &str, topic: &str) {
        match self {
            Self::ApplicationError(description) => {
                error!(
                    monotonic_counter.processing_errors = 1,
                    service = %service,
                    topic = %topic,
                    description
                );
            }
            Self::ExternalError(description) => {
                error!(monotonic_counter.external_errors = 1, service = %service, topic = %topic, description);
            }
            Self::ServerError(description) => {
                error!(monotonic_counter.runtime_errors = 1, service = %service, description);
            }
            Self::InvalidInput(description) => {
                warn!(
                    monotonic_counter.parsing_errors = 1,
                    service = %service,
                    topic = %topic,
                    description
                );
            }
            Self::NotFound(description) => {
                info!(description);
            }
            Self::Outdated(description) => {
                info!(monotonic_counter.stale_data = 1, service = %service, topic = %topic, description);
            }
            Self::Other(description) => {
                info!(monotonic_counter.other_errors = 1, service = %service, description);
            }
        }
    }

    pub fn from_string(raw: String) -> Self {
        let obj: serde_json::Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => return Self::Other(raw),
        };

        let code = obj.get("code").and_then(Value::as_u64).unwrap_or(500);
        let description =
            obj.get("description").and_then(Value::as_str).unwrap_or(&raw).to_string();

        match code {
            400 => Self::InvalidInput(description),
            404 => Self::NotFound(description),
            410 => Self::Outdated(description),
            502 => Self::ExternalError(description),
            503 => Self::ApplicationError(description),
            500 => Self::ServerError(description),
            _ => Self::Other(description),
        }
    }
}

impl From<wasmtime::Error> for Error {
    fn from(err: wasmtime::Error) -> Self {
        Self::from_string(err.root_cause().to_string())
    }
}

impl From<wasmtime_wasi::ResourceTableError> for Error {
    fn from(err: wasmtime_wasi::ResourceTableError) -> Self {
        Self::ServerError(err.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::ServerError(err.to_string())
    }
}
