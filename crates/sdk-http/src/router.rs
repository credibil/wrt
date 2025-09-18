use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use anyhow::{Result, anyhow, bail};
use axum::body::Body;
use axum::http::{Request as HttpRequest, Response as HttpResponse};
use futures::StreamExt;
use futures::executor::block_on;
use http::{HeaderName, HeaderValue, Uri};
use percent_encoding::percent_decode_str;
use tower::ServiceExt;
use wasi::http::types::{
    ErrorCode, Fields, Headers, IncomingRequest, Method as WasiMethod, OutgoingBody,
    OutgoingResponse,
};

/// Serve an incoming request using the provided router.
///
/// # Errors
///
/// Returns a [`wasi::http::types::ErrorCode`] if the request could not be served.
pub fn serve(
    router: axum::Router, request: IncomingRequest,
) -> Result<OutgoingResponse, ErrorCode> {
    // forward request to axum `Router` to handle
    let http_req =
        Request(request).into_http().map_err(|e| error!("issue converting request: {e}"))?;
    let http_resp = block_on(async { router.oneshot(http_req).await })
        .map_err(|e| error!("issue processing request: {e}"))?;

    // convert axum `Response` to `OutgoingResponse`
    let response:Response = http_resp.try_into()
        .map_err(|e| error!("issue converting response: {e}"))?;
    Ok(response.0)
}

struct Request(IncomingRequest);

impl Request {
    // fn new(request: IncomingRequest) -> Self {
    //     Request(request)
    // }

    fn method(&self) -> Method {
        Method(self.0.method())
    }

    fn headers(&self) -> Fields {
        self.0.headers()
    }

    fn uri(&self) -> Uri {
        let p_and_q = self.0.path_with_query().unwrap_or_default();
        let decoded = percent_decode_str(p_and_q.as_str()).decode_utf8_lossy();
        decoded.parse::<Uri>().unwrap_or_else(|_| Uri::default())
    }

    fn body(&self) -> Result<Vec<u8>> {
        let body = self.0.consume().map_err(|()| anyhow!("issue consuming request body"))?;
        let stream = body.stream().map_err(|()| anyhow!("issue getting body stream"))?;

        let mut buffer = Vec::new();
        while let Ok(bytes) = stream.blocking_read(4096)
            && !bytes.is_empty()
        {
            buffer.extend_from_slice(&bytes);
        }
        drop(stream);

        Ok(buffer)
    }

    fn into_http(self) -> Result<HttpRequest<Body>> {
        self.try_into()
    }
}

impl TryFrom<Request> for HttpRequest<Body> {
    type Error = anyhow::Error;

    fn try_from(request: Request) -> Result<Self, Self::Error> {
        let method = request.method().to_string();
        let headers = request.headers();
        let uri = request.uri();
        let bytes = request.body()?;
        let body = if bytes.is_empty() { Body::empty() } else { Body::from(bytes) };

        let mut http_req = HttpRequest::builder().method(method.as_str()).uri(&uri).body(body)?;
        for (key, value) in headers.entries() {
            let Ok(name) = HeaderName::from_str(&key) else { continue };
            let Ok(value) = HeaderValue::from_bytes(&value) else { continue };
            http_req.headers_mut().insert(name, value);
        }
        if http_req.headers().get("host").is_none()
            && let Some(authority) = uri.authority()
            && let Ok(value) = HeaderValue::from_str(authority.as_str())
        {
            http_req.headers_mut().insert("host", value);
        }

        Ok(http_req)
    }
}

struct Response(OutgoingResponse);

impl TryFrom<HttpResponse<Body>> for Response {
    type Error = anyhow::Error;

    fn try_from(value: HttpResponse<Body>) -> std::result::Result<Response, Self::Error> {
        let headers = Headers::new();
        for (key, value) in value.headers() {
            headers.set(key.as_str(), &[value.as_bytes().to_vec()])?;
        }
        let response = OutgoingResponse::new(headers);
        response
            .set_status_code(value.status().as_u16())
            .map_err(|()| error!("issue setting status code"))?;

        // write `OutgoingBody`
        let http_body = value.into_body();
        let mut http_stream = http_body.into_data_stream();
        let out_body = response.body().map_err(|()| error!("issue getting outgoing body"))?;
        let out_stream = out_body.write().map_err(|()| error!("issue getting body stream"))?;
        let pollable = out_stream.subscribe();
        while let Some(Ok(chunk)) = block_on(async { http_stream.next().await }) {
            pollable.block();
            out_stream.check_write().map_err(|e| error!("issue checking write: {e}"))?;

            if let Err(e) = out_stream.write(&chunk) {
                bail!("issue writing to stream: {e}");
            }
        }
        if let Err(e) = out_stream.flush() {
            bail!("issue flushing stream: {e}");
        }
        pollable.block();

        // check for errors
        if let Err(e) = out_stream.check_write() {
            bail!("issue writing to stream: {e}");
        }
        drop(pollable);
        drop(out_stream);

        if let Err(e) = OutgoingBody::finish(out_body, None) {
            bail!("issue finishing body: {e}");
        }

        Ok(Response(response))
    }
}

#[derive(Debug, Clone)]
struct Method(WasiMethod);

impl Display for Method {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let method = match &self.0 {
            WasiMethod::Get => "GET",
            WasiMethod::Post => "POST",
            WasiMethod::Put => "PUT",
            WasiMethod::Delete => "DELETE",
            WasiMethod::Patch => "PATCH",
            WasiMethod::Head => "HEAD",
            WasiMethod::Options => "OPTIONS",
            WasiMethod::Trace => "TRACE",
            WasiMethod::Connect => "CONNECT",
            WasiMethod::Other(s) => s,
        };
        write!(f, "{method}")
    }
}

macro_rules! error {
    ($fmt:expr, $($arg:tt)*) => {
        ErrorCode::InternalError(Some(format!($fmt, $($arg)*)))
    };
     ($err:expr $(,)?) => {
        ErrorCode::InternalError(Some(format!($err)))
    };
}
pub(crate) use error;
