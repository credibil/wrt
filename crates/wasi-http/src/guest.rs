//! # HTTP SDK
//!
//! Wasm component (guest) HTTP SDK.

mod cache;
mod incoming;
mod outgoing;

pub use axum;
use http::header::{self, HeaderName};

pub use self::incoming::*;
pub use self::outgoing::*;

/// Set of `[http::header::HeaderName]`, that are forbidden by default
/// for requests and responses originating in the guest.
pub const DEFAULT_FORBIDDEN_HEADERS: [HeaderName; 9] = [
    header::CONNECTION,
    HeaderName::from_static("keep-alive"),
    header::PROXY_AUTHENTICATE,
    header::PROXY_AUTHORIZATION,
    HeaderName::from_static("proxy-connection"),
    header::TRANSFER_ENCODING,
    header::UPGRADE,
    header::HOST,
    HeaderName::from_static("http2-settings"),
];
