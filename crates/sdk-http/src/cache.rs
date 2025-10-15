//! Cache header parsing and cache get/put

use anyhow::{anyhow, bail};
use bytes::Bytes;
use http::Response;
use serde::{Deserialize, Serialize};
use wasi::http::types::Headers;
use wit_bindings::keyvalue::store;

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
    control: CacheControl,
}

impl Cache {
    /// Parse cache-related headers from the provided `Headers`.
    pub fn new(headers: &Headers) -> anyhow::Result<Self> {
        let mut control = CacheControl::default();
        let cache_header = headers.get(http::header::CACHE_CONTROL.as_str());
        if cache_header.is_empty() {
            return Ok(Self { control });
        }
        // Use only the first Cache-Control header if multiple are present.
        let Some(cache_header) = cache_header.first() else {
            return Ok(Self { control });
        };
        if cache_header.is_empty() {
            bail!("Cache-Control header is empty");
        }

        let raw = String::from_utf8(cache_header.clone())
            .map_err(|e| anyhow!("issue parsing Cache-Control header: {e}"))?;

        for directive in raw.split(',') {
            let directive = directive.trim();
            if directive.is_empty() {
                continue;
            }

            let directive_lower = directive.to_ascii_lowercase();

            if directive_lower == "no-store" {
                if control.no_cache || control.max_age > 0 {
                    bail!("`no-store` cannot be combined with other cache directives");
                }
                control.no_store = true;
                continue;
            }

            if directive_lower == "no-cache" {
                if control.no_store {
                    bail!("`no-cache` cannot be combined with `no-store`");
                }
                control.no_cache = true;
                continue;
            }

            if directive_lower.starts_with("max-age=") {
                if control.no_store {
                    bail!("`max-age` cannot be combined with `no-store`");
                }

                let Ok(seconds) = directive[8..].trim().parse() else {
                    bail!("`max-age` directive is malformed");
                };
                control.max_age = seconds;
            }
            // ignore other directives
        }

        if !control.no_store {
            let etag = headers.get(http::header::IF_NONE_MATCH.as_str());
            if etag.is_empty() {
                bail!(
                    "`If-None-Match` header required when using `Cache-Control: max-age` or `no-cache`"
                );
            }
            // Use only the first ETag header if multiple are present.
            let Some(etag) = etag.first() else {
                bail!(
                    "`If-None-Match` header required when using `Cache-Control: max-age` or `no-cache`"
                );
            };
            if etag.is_empty() {
                bail!("`If-None-Match` header is empty");
            }
            let etag = String::from_utf8(etag.clone())
                .map_err(|e| anyhow!("issue parsing `If-None-Match` header: {e}"))?;
            if etag.contains(',') {
                bail!("multiple `etag` values in `If-None-Match` header are not supported");
            }
            if etag.starts_with("W/") {
                bail!("weak `etag` values in `If-None-Match` header are not supported");
            }
            control.etag = etag;
        }
        Ok(Self { control })
    }

    /// Return true if a response should be fetched from cache first.
    pub fn should_use_cache(&self) -> bool {
        !self.control.no_cache && !self.control.no_store && !self.control.etag.is_empty()
    }

    /// Return true if the response should be fetched from the origin server
    /// before checking the cache.
    pub fn should_fetch(&self) -> bool {
        self.control.no_cache || self.control.no_store
    }

    /// Return true if the response should be cached.
    pub fn should_store(&self) -> bool {
        !self.control.no_store && !self.control.etag.is_empty() && self.control.max_age > 0
    }

    /// Put the response into cache.
    ///
    /// # Errors
    /// * serialization errors
    /// * cache storage errors
    pub fn put(&self, response: Response<Bytes>) -> anyhow::Result<()> {
        if !self.should_store() {
            return Ok(());
        }

        Ok(())
    }

    /// Get a cached response.
    ///
    /// Returns None if no cached response is found.
    ///
    /// # Errors
    /// * cache retrieval errors
    /// * deserialization errors
    pub fn get(&self) -> anyhow::Result<Option<Response<Bytes>>> {
        if self.control.etag.is_empty() {
            bail!("no etag to use as cache key");
        }

        todo!()
    }
}

#[derive(Deserialize, Serialize)]
struct SerializableResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

fn serialize_response(response: &Response<Bytes>) -> anyhow::Result<Bytes> {
    todo!()
}

fn deserialize_response(data: &[u8]) -> anyhow::Result<Response<Bytes>> {
    todo!()
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
