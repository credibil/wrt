//! Cache header parsing and cache get/put

use anyhow::{Context, Result, anyhow, bail};
use bincode::{Decode, Encode, config};
use bytes::Bytes;
use http::header::{CACHE_CONTROL, IF_NONE_MATCH};
use http::{Request, Response};
use http_body::Body;
use wasi_keyvalue::store;

pub const CACHE_BUCKET: &str = "default-cache";

#[derive(Debug, Default)]
pub struct Cache {
    control: Control,
    bucket: String,
}

impl Cache {
    /// Create a Cache instance from the request headers, if caching is indicated.
    pub fn maybe_from(request: &Request<impl Body>) -> Result<Option<Self>> {
        let headers = request.headers();
        if headers.get(CACHE_CONTROL).is_none() {
            tracing::debug!("no Cache-Control header present");
            return Ok(None);
        }

        let control = Control::try_from(headers).context("issue parsing Cache-Control headers")?;

        Ok(Some(Self {
            bucket: CACHE_BUCKET.to_string(),
            control,
        }))
    }

    /// Optionally set the cache bucket name.
    pub fn with_bucket(mut self, bucket: &str) -> Self {
        self.bucket = bucket.to_string();
        self
    }

    /// Get a cached response.
    ///
    /// # Errors
    ///
    /// * cache retrieval errors
    /// * deserialization errors
    pub fn maybe_get(&self) -> Result<Option<Response<Bytes>>> {
        let ctrl = &self.control;
        if ctrl.no_cache || ctrl.no_store || ctrl.etag.is_empty() {
            tracing::debug!("cache is disabled");
            return Ok(None);
        }

        // get data from keyvalue store
        let bucket = store::open(&self.bucket)
            .map_err(|e| anyhow!("opening cache bucket `{}`: {e}", &self.bucket))?;
        let Some(data) = bucket
            .get(&self.control.etag)
            .map_err(|e| anyhow!("retrieving cached response: {e}"))?
        else {
            return Ok(None);
        };

        deserialize(&data).map(Some)
    }

    /// Put the response into cache.
    ///
    /// # Errors
    ///
    /// * serialization errors
    /// * cache storage errors
    pub fn maybe_put(&self, response: &Response<Bytes>) -> Result<()> {
        let ctrl = &self.control;
        if ctrl.no_store || ctrl.etag.is_empty() || ctrl.max_age == 0 {
            return Ok(());
        }
        tracing::debug!("caching response with etag `{}`", &ctrl.etag);

        let value = serialize(response)?;
        let bucket = store::open(&self.bucket)
            .map_err(|e| anyhow!("opening cache bucket `{}`: {e}", &self.bucket))?;
        bucket.set(&ctrl.etag, &value).map_err(|e| anyhow!("storing response in cache: {e}"))
    }
}

#[derive(Clone, Debug, Default)]
struct Control {
    // If true, make the HTTP request and then update the cache with the
    // response.
    no_cache: bool,

    // If true, make the HTTP request and do not cache the response.
    no_store: bool,

    // Length of time to cache the response in seconds.
    max_age: u64,

    // ETag to use as the cache key, derived from the `If-None-Match` header.
    etag: String,
}

// impl Default for Control {
//     fn default() -> Self {
//         Self {
//             no_cache: true,
//             no_store: false,
//             max_age: 0,
//             etag: String::new(),
//         }
//     }
// }

impl TryFrom<&http::HeaderMap> for Control {
    type Error = anyhow::Error;

    fn try_from(headers: &http::HeaderMap) -> Result<Self> {
        let mut control = Self::default();

        let cache_control = headers.get(http::header::CACHE_CONTROL);
        let Some(cache_control) = cache_control else {
            tracing::debug!("no Cache-Control header present");
            return Ok(control);
        };

        if cache_control.is_empty() {
            bail!("Cache-Control header is empty");
        }

        for directive in cache_control.to_str()?.split(',') {
            let directive = directive.trim().to_ascii_lowercase();
            if directive.is_empty() {
                continue;
            }

            if directive == "no-store" {
                if control.no_cache || control.max_age > 0 {
                    bail!("`no-store` cannot be combined with other cache directives");
                }
                control.no_store = true;
                continue;
            }

            if directive == "no-cache" {
                if control.no_store {
                    bail!("`no-cache` cannot be combined with `no-store`");
                }
                control.no_cache = true;
                continue;
            }

            if let Some(value) = directive.strip_prefix("max-age=") {
                if control.no_store {
                    bail!("`max-age` cannot be combined with `no-store`");
                }
                let Ok(max_age) = value.trim().parse() else {
                    bail!("`max-age` directive is malformed");
                };
                control.max_age = max_age;
            }

            // ignore other directives
        }

        if !control.no_store {
            let Some(etag) = headers.get(IF_NONE_MATCH) else {
                bail!(
                    "`If-None-Match` header required when using `Cache-Control: max-age` or `no-cache`"
                );
            };
            if etag.is_empty() {
                bail!("`If-None-Match` header is empty");
            }

            let etag_str = etag.to_str()?;
            if etag_str.contains(',') {
                bail!("multiple `etag` values in `If-None-Match` header are not supported");
            }
            if etag_str.starts_with("W/") {
                bail!("weak `etag` values in `If-None-Match` header are not supported");
            }
            control.etag = etag_str.to_string();
        }

        Ok(control)
    }
}

fn serialize(response: &Response<Bytes>) -> Result<Vec<u8>> {
    let ser = Serialized::try_from(response)?;
    bincode::encode_to_vec(&ser, config::standard())
        .map_err(|e| anyhow!("serializing response: {e}"))
}

fn deserialize(data: &[u8]) -> Result<Response<Bytes>> {
    let (ser, _): (Serialized, _) = bincode::decode_from_slice(data, config::standard())
        .map_err(|e| anyhow!("deserializing cached response: {e}"))?;
    Response::<Bytes>::try_from(ser)
}

#[derive(Decode, Encode)]
struct Serialized {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl TryFrom<&Response<Bytes>> for Serialized {
    type Error = anyhow::Error;

    fn try_from(response: &Response<Bytes>) -> Result<Self> {
        Ok(Self {
            status: response.status().as_u16(),
            headers: response
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
                .collect(),
            body: response.body().to_vec(),
        })
    }
}

impl TryFrom<Serialized> for Response<Bytes> {
    type Error = anyhow::Error;

    fn try_from(s: Serialized) -> Result<Self> {
        let mut response = Response::builder().status(s.status);
        for (k, v) in s.headers {
            response = response.header(k, v);
        }
        response
            .body(Bytes::from(s.body))
            .map_err(|e| anyhow!("building response from cached data: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use http::HeaderMap;
    use http::header::{CACHE_CONTROL, IF_NONE_MATCH};

    use super::*;

    #[test]
    fn returns_none_when_header_missing() {
        let headers = HeaderMap::new();
        let control = Control::try_from(&headers).expect("should parse");

        assert!(control.no_cache);
        assert!(!control.no_store);
        assert_eq!(control.max_age, 0);
        assert!(control.etag.is_empty());
    }

    #[test]
    fn parses_max_age_with_etag() {
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "max-age=120".parse().unwrap());
        headers.append(IF_NONE_MATCH, "\"strong-etag\"".parse().unwrap());

        let control = Control::try_from(&headers).expect("should parse");

        assert!(!control.no_store);
        assert_eq!(control.max_age, 120);
        assert_eq!(control.etag, "\"strong-etag\"");
    }

    #[test]
    fn requires_etag_when_store_enabled() {
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "no-cache".parse().unwrap());

        let Err(_) = Control::try_from(&headers) else {
            panic!("expected missing etag error");
        };
    }

    #[test]
    fn rejects_conflicting_directives() {
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "no-cache, max-age=10".parse().unwrap());
        headers.append(IF_NONE_MATCH, "\"etag\"".parse().unwrap());

        let Err(_) = Control::try_from(&headers) else {
            panic!("expected conflicting directives error");
        };
    }

    #[test]
    fn rejects_weak_etag_value() {
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "no-cache".parse().unwrap());
        headers.append(IF_NONE_MATCH, "W/\"weak-etag\"".parse().unwrap());

        let Err(_) = Control::try_from(&headers) else {
            panic!("expected weak etag rejection");
        };
    }

    #[test]
    fn rejects_multiple_etag_values() {
        let mut headers = HeaderMap::new();
        headers.append(CACHE_CONTROL, "no-cache".parse().unwrap());
        headers.append(IF_NONE_MATCH, "\"etag1\", \"etag2\"".parse().unwrap());

        let Err(_) = Control::try_from(&headers) else {
            panic!("expected multiple etag values rejection");
        };
    }
}
