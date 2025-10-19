//! Cache header parsing and cache get/put

use anyhow::{anyhow, bail};
use bincode::{Decode, Encode, config};
use bytes::Bytes;
use http::Response;
use wasi::http::types::Headers;

use crate::generated::wasi::keyvalue::store;

#[derive(Clone, Debug, Default)]
struct CacheControl {
    // If true, make the HTTP request and then update the cache with the
    // response.
    pub no_cache: bool,

    // If true, make the HTTP request and do not cache the response.
    pub no_store: bool,

    // Length of time to cache the response in seconds.
    pub max_age: u64,

    // ETag to use as the cache key, derived from the `If-None-Match` header.
    pub etag: String,
}

#[derive(Debug, Default)]
pub struct Cache {
    bucket: String,
    control: CacheControl,
}

impl Cache {
    /// Create a new cache manager with the provided bucket name.
    pub fn new(bucket: &str) -> Self {
        let bucket = if bucket.is_empty() { "default" } else { bucket };
        Self {
            bucket: bucket.to_string(),
            control: CacheControl::default(),
        }
    }

    /// Parse cache-related headers from the provided `Headers`.
    pub fn headers(&mut self, headers: &Headers) -> anyhow::Result<()> {
        let mut control = CacheControl::default();
        let cache_header = headers.get(http::header::CACHE_CONTROL.as_str());
        if cache_header.is_empty() {
            tracing::debug!("no Cache-Control header present");
            return Ok(());
        }
        // Use only the first Cache-Control header if multiple are present.
        let Some(cache_header) = cache_header.first() else {
            return Ok(());
        };
        if cache_header.is_empty() {
            let err = "Cache-Control header is empty";
            bail!(err);
        }

        let raw = match String::from_utf8(cache_header.clone()) {
            Ok(s) => s,
            Err(e) => {
                let err = format!("issue parsing Cache-Control header: {e}");
                return Err(anyhow!(err));
            }
        };

        for directive in raw.split(',') {
            let directive = directive.trim();
            if directive.is_empty() {
                continue;
            }

            let directive_lower = directive.to_ascii_lowercase();

            if directive_lower == "no-store" {
                if control.no_cache || control.max_age > 0 {
                    let err = "`no-store` cannot be combined with other cache directives";
                    bail!(err);
                }
                control.no_store = true;
                continue;
            }

            if directive_lower == "no-cache" {
                if control.no_store {
                    let err = "`no-cache` cannot be combined with `no-store`";
                    bail!(err);
                }
                control.no_cache = true;
                continue;
            }

            if directive_lower.starts_with("max-age=") {
                if control.no_store {
                    let err = "`max-age` cannot be combined with `no-store`";
                    bail!(err);
                }

                let Ok(seconds) = directive[8..].trim().parse() else {
                    let err = "`max-age` directive is malformed";
                    bail!(err);
                };
                control.max_age = seconds;
            }
            // ignore other directives
        }

        if !control.no_store {
            let etag = headers.get(http::header::IF_NONE_MATCH.as_str());
            if etag.is_empty() {
                let err = "`If-None-Match` header required when using `Cache-Control: max-age` or `no-cache`";
                bail!(err);
            }
            // Use only the first ETag header if multiple are present.
            let Some(etag) = etag.first() else {
                let err = "cannot parse first `If-None-Match` header";
                bail!(err);
            };
            if etag.is_empty() {
                let err = "`If-None-Match` header is empty";
                bail!(err);
            }
            let etag = match String::from_utf8(etag.clone()) {
                Ok(etag) => etag,
                Err(e) => {
                    let err = format!("issue parsing `If-None-Match` header: {e}");
                    return Err(anyhow!(err));
                }
            };
            if etag.contains(',') {
                let err = "multiple `etag` values in `If-None-Match` header are not supported";
                bail!(err);
            }
            if etag.starts_with("W/") {
                let err = "weak `etag` values in `If-None-Match` header are not supported";
                bail!(err);
            }
            control.etag = etag;
        }
        self.control = control;
        Ok(())
    }

    /// Return true if a response should be fetched from cache first.
    pub const fn should_use_cache(&self) -> bool {
        !self.control.no_cache && !self.control.no_store && !self.control.etag.is_empty()
    }

    /// Return true if the response should be cached.
    pub const fn should_store(&self) -> bool {
        !self.control.no_store && !self.control.etag.is_empty() && self.control.max_age > 0
    }

    /// Put the response into cache.
    ///
    /// # Errors
    /// * serialization errors
    /// * cache storage errors
    pub fn put(&self, response: &Response<Bytes>) -> anyhow::Result<()> {
        if !self.should_store() {
            return Ok(());
        }
        tracing::debug!("caching response with etag `{}`", &self.control.etag);
        let value = serialize_response(response)?;
        let bucket = store::open(&self.bucket)
            .map_err(|e| anyhow!("opening cache bucket `{}`: {e}", &self.bucket))?;
        bucket
            .set(&self.control.etag, &value)
            .map_err(|e| anyhow!("storing response in cache: {e}"))
    }

    /// Get a cached response.
    ///
    /// Returns None if no cached response is found.
    ///
    /// # Errors
    /// * cache retrieval errors
    /// * deserialization errors
    pub fn get(&self) -> anyhow::Result<Option<Response<Bytes>>> {
        tracing::debug!("checking cache for etag `{}`", &self.control.etag);
        if self.control.etag.is_empty() {
            bail!("no etag to use as cache key");
        }
        let bucket = store::open(&self.bucket)
            .map_err(|e| anyhow!("opening cache bucket `{}`: {e}", &self.bucket))?;
        let data = bucket
            .get(&self.control.etag)
            .map_err(|e| anyhow!("retrieving cached response: {e}"))?;
        if let Some(data) = data {
            let response = deserialize_response(&data)?;
            Ok(Some(response))
        } else {
            Ok(None)
        }
    }
}

#[derive(Decode, Encode)]
struct SerializableResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

fn serialize_response(response: &Response<Bytes>) -> anyhow::Result<Vec<u8>> {
    let serializable = SerializableResponse {
        status: response.status().as_u16(),
        headers: response
            .headers()
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or_default().to_string()))
            .collect(),
        body: response.body().to_vec(),
    };
    bincode::encode_to_vec(&serializable, config::standard())
        .map_err(|e| anyhow!("serializing response for cache: {e}"))
}

fn deserialize_response(data: &[u8]) -> anyhow::Result<Response<Bytes>> {
    let (serializable, _): (SerializableResponse, _) =
        bincode::decode_from_slice(data, config::standard())
            .map_err(|e| anyhow!("deserializing cached response: {e}"))?;
    let mut response = Response::builder().status(serializable.status);
    for (k, v) in serializable.headers {
        response = response.header(&k, &v);
    }
    response
        .body(Bytes::from(serializable.body))
        .map_err(|e| anyhow!("building response from cached data: {e}"))
}

// #[cfg(test)]
// mod tests {
//     use http::header::{CACHE_CONTROL, IF_NONE_MATCH};

//     use super::*;

//     #[test]
//     fn returns_none_when_header_missing() {
//         let headers = Headers::new();
//         let result = cache_headers(&headers).expect("parsing succeeds without header");
//         assert!(result.is_none());
//     }

//     #[test]
//     fn parses_max_age_with_etag() {
//         let headers = Headers::new();
//         headers.append(CACHE_CONTROL.as_str(), b"max-age=120").expect("cache control header set");
//         headers.append(IF_NONE_MATCH.as_str(), b"\"strong-etag\"").expect("if-none-match header set");

//         let control =
//             cache_headers(&headers).expect("parsing succeeds").expect("cache control present");

//         assert!(!control.no_store);
//         assert_eq!(control.max_age, 120);
//         assert_eq!(control.etag, "\"strong-etag\"");
//     }

//     #[test]
//     fn requires_etag_when_store_enabled() {
//         let headers = Headers::new();
//         headers.append(CACHE_CONTROL.as_str(), b"no-cache").expect("cache control header set");

//         let Err(_) = cache_headers(&headers) else {
//             panic!("expected missing etag error");
//         };
//     }

//     #[test]
//     fn rejects_conflicting_directives() {
//         let headers = Headers::new();
//         headers.append(CACHE_CONTROL.as_str(), b"no-cache, max-age=10").expect("cache control header set");
//         headers.append(IF_NONE_MATCH.as_str(), b"\"etag\"").expect("if-none-match header set");

//         let Err(_) = cache_headers(&headers) else {
//             panic!("expected conflicting directives error");
//         };
//     }

//     #[test]
//     fn rejects_weak_etag_value() {
//         let headers = Headers::new();
//         headers.append(CACHE_CONTROL.as_str(), b"no-cache").expect("cache control header set");
//         headers.append(IF_NONE_MATCH.as_str(), b"W/\"weak-etag\"").expect("if-none-match header set");

//         let Err(_) = cache_headers(&headers) else {
//             panic!("expected weak etag rejection");
//         };
//     }

//     #[test]
//     fn rejects_multiple_etag_values() {
//         let headers = Headers::new();
//         headers.append(CACHE_CONTROL.as_str(), b"no-cache").expect("cache control header set");
//         headers.append(IF_NONE_MATCH.as_str(), b"\"etag1\", \"etag2\"").expect("if-none-match header set");

//         let Err(_) = cache_headers(&headers) else {
//             panic!("expected multiple etag values rejection");
//         };
//     }
// }
