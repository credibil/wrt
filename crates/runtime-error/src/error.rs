use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// HTTP status code for "I'm a teapot"
/// Used as a default for unknown errors
const TEA_POT: u64 = 418;

#[derive(Clone, Debug, Deserialize, Eq, Error, PartialEq, Serialize)]
pub enum Error {
    /// Bad Request (400)
    /// Reserved for parsing/validation errors
    #[error("{{\"code\": 400, \"description\": \"{0}\"}}")]
    BadRequest(String),

    /// Unauthorized (401)
    /// Reserved for authorization errors
    #[error("{{\"code\": 401, \"description\": \"{0}\"}}")]
    Unauthorized(String),

    /// Not Found (404)
    /// Reserved for missing data errors
    #[error("{{\"code\": 404, \"description\": \"{0}\"}}")]
    NotFound(String),

    /// Gone (410)
    /// Reserved for stale data errors
    #[error("{{\"code\": 410, \"description\": \"{0}\"}}")]
    Gone(String),

    /// I'm a teapot (418)
    /// Reserved for all other/unknown errors
    #[error("{{\"code\": 418, \"description\": \"{0}\"}}")]
    ImATeaPot(String),

    /// Internal Server Error (500)
    /// Reserved for runtime errors
    #[error("{{\"code\": 500, \"description\": \"{0}\"}}")]
    ServerError(String),

    /// Bad Gateway (502)
    /// Reserved for upstream service errors (e.g. external sources, API calls, databases, etc.)
    #[error("{{\"code\": 502, \"description\": \"{0}\"}}")]
    BadGateway(String),

    /// Service Unavailable (503)
    /// Reserved for Application-level errors
    #[error("{{\"code\": 503, \"description\": \"{0}\"}}")]
    ServiceUnavailable(String),
}

impl Error {
    /// Returns the error code.
    #[must_use]
    pub const fn code(&self) -> u64 {
        match self {
            Self::BadRequest(_) => 400,
            Self::Unauthorized(_) => 401,
            Self::NotFound(_) => 404,
            Self::Gone(_) => 410,
            Self::ImATeaPot(_) => 418,
            Self::ServerError(_) => 500,
            Self::BadGateway(_) => 502,
            Self::ServiceUnavailable(_) => 503,
        }
    }

    /// Returns the error description.
    #[must_use]
    pub fn description(&self) -> &str {
        match self {
            Self::BadRequest(desc)
            | Self::Unauthorized(desc)
            | Self::NotFound(desc)
            | Self::Gone(desc)
            | Self::ImATeaPot(desc)
            | Self::ServerError(desc)
            | Self::BadGateway(desc)
            | Self::ServiceUnavailable(desc) => desc,
        }
    }
}

impl FromStr for Error {
    type Err = Self;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Ok(Value::Object(obj)) = serde_json::from_str(s) else {
            return Ok(Self::ImATeaPot(s.to_string()));
        };

        let code = obj.get("code").and_then(Value::as_u64).unwrap_or(TEA_POT);
        let description = obj.get("description").and_then(Value::as_str).unwrap_or(s).to_string();

        let error = match code {
            400 => Self::BadRequest(description),
            401 => Self::Unauthorized(description),
            404 => Self::NotFound(description),
            410 => Self::Gone(description),
            502 => Self::BadGateway(description),
            503 => Self::ServiceUnavailable(description),
            500 => Self::ServerError(description),
            _ => Self::ImATeaPot(description),
        };

        Ok(error)
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Self::from_str(err.root_cause().to_string().as_str()).unwrap_or_else(|_| {
            let stack = err.chain().fold(String::new(), |cause, e| format!("{cause} -> {e}"));
            let stack = stack.trim_start_matches(" -> ").to_string();
            Self::ServiceUnavailable(stack)
        })
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::BadRequest(err.to_string())
    }
}
