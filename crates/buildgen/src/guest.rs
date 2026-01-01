pub mod expand;
pub mod generate;

use proc_macro2::Span;
use quote::format_ident;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Path, Result, Token, Type, parse_str};

pub struct Input {
    pub owner: LitStr,
    pub provider: Type,
    pub http: Option<Vec<Route>>,
    pub messaging: Option<Vec<Topic>>,
}

pub struct Route {
    pub path: LitStr,
    pub params: Vec<Ident>,
    pub method: Ident,
    pub request: Type,
    pub reply: Type,
}

pub struct Topic {
    pub pattern: LitStr,
    pub message: Type,
    pub handler: Option<Path>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        syn::braced!(content in input);

        // let mut out = Self::default();
        let mut owner: Option<LitStr> = None;
        let mut provider: Option<Type> = None;
        let mut http: Option<Vec<Route>> = None;
        let mut messaging: Option<Vec<Topic>> = None;

        while !content.is_empty() {
            let ident: Ident = content.parse()?;
            content.parse::<Token![:]>()?;

            match ident.to_string().as_str() {
                "owner" => {
                    owner = content.parse()?;
                }
                "provider" => {
                    provider = Some(content.parse()?);
                }
                "http" => {
                    let list;
                    syn::bracketed!(list in content);
                    http = Some(parse_routes(&list)?);
                }
                "messaging" => {
                    let list;
                    syn::bracketed!(list in content);
                    messaging = Some(parse_topics(&list)?);
                }
                _ => {
                    return Err(syn::Error::new(
                        ident.span(),
                        "unknown field; expected `owner`, `provider`, `http`, or `messaging`",
                    ));
                }
            }

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        let Some(owner) = owner else {
            return Err(syn::Error::new(Span::call_site(), "missing `owner`"));
        };
        let Some(provider) = provider else {
            return Err(syn::Error::new(Span::call_site(), "missing `provider`"));
        };

        Ok(Self {
            owner,
            provider,
            http,
            messaging,
        })
    }
}

fn parse_routes(input: ParseStream) -> Result<Vec<Route>> {
    let mut routes = Vec::new();
    while !input.is_empty() {
        let path: LitStr = input.parse()?;
        input.parse::<Token![:]>()?;

        let settings;
        syn::braced!(settings in input);

        let mut method: Option<Ident> = None;
        let mut request: Option<Type> = None;
        let mut reply: Option<Type> = None;

        while !settings.is_empty() {
            let key: Ident = settings.parse()?;
            settings.parse::<Token![:]>()?;

            if key == "method" {
                method = Some(settings.parse()?);
            } else if key == "request" {
                request = Some(settings.parse()?);
            } else if key == "reply" {
                reply = Some(settings.parse()?);
            } else {
                return Err(syn::Error::new(
                    key.span(),
                    "unknown http field; expected `method`, `request`, or `reply`",
                ));
            }

            if settings.peek(Token![,]) {
                settings.parse::<Token![,]>()?;
            }
        }

        // validate required fields
        let method = if let Some(method) = method {
            let method_str = method.to_string().to_lowercase();
            match method_str.as_str() {
                "get" | "post" => format_ident!("{method_str}"),
                _ => {
                    return Err(syn::Error::new(
                        method.span(),
                        "unsupported http method; expected `get` or `post`",
                    ));
                }
            }
        } else {
            format_ident!("get")
        };

        let Some(request) = request else {
            return Err(syn::Error::new(path.span(), "http route is missing `request`"));
        };
        let Some(reply) = reply else {
            return Err(syn::Error::new(path.span(), "http route is missing `reply`"));
        };
        let params = parse_path_params(&path)?;

        routes.push(Route {
            path,
            params,
            method,
            request,
            reply,
        });

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
    }
    Ok(routes)
}

fn parse_path_params(path: &LitStr) -> Result<Vec<Ident>> {
    let s = path.value();
    let mut params = Vec::<Ident>::new();

    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                // Find the closing `}`.
                let start = i + 1;
                i += 1;
                while i < bytes.len() && bytes[i] != b'}' {
                    if bytes[i] == b'{' {
                        return Err(syn::Error::new(
                            path.span(),
                            "invalid http path: nested `{` in path params",
                        ));
                    }
                    i += 1;
                }
                if i >= bytes.len() {
                    return Err(syn::Error::new(
                        path.span(),
                        "invalid http path: unclosed `{` in path params",
                    ));
                }

                let end = i;
                let name = s[start..end].trim();
                if name.is_empty() {
                    return Err(syn::Error::new(
                        path.span(),
                        "invalid http path: empty `{}` path parameter",
                    ));
                }

                let ident: Ident = parse_str(name).map_err(|e| {
                    syn::Error::new(
                        path.span(),
                        format!(
                            "invalid http path parameter name `{name}`; must be a valid Rust identifier: {e}"
                        ),
                    )
                })?;

                if params.iter().any(|p| p == &ident) {
                    return Err(syn::Error::new(
                        path.span(),
                        format!("duplicate http path parameter `{ident}`"),
                    ));
                }

                params.push(ident);
                i += 1; // consume `}`
            }
            b'}' => {
                return Err(syn::Error::new(
                    path.span(),
                    "invalid http path: stray `}` in path params",
                ));
            }
            _ => i += 1,
        }
    }

    Ok(params)
}

fn parse_topics(input: ParseStream) -> Result<Vec<Topic>> {
    let mut topics = Vec::new();
    while !input.is_empty() {
        let pattern: LitStr = input.parse()?;
        input.parse::<Token![:]>()?;

        let settings;
        syn::braced!(settings in input);

        let mut message: Option<Type> = None;
        let mut handler: Option<Path> = None;

        while !settings.is_empty() {
            let key: Ident = settings.parse()?;
            settings.parse::<Token![:]>()?;

            if key == "message" {
                message = Some(settings.parse()?);
            } else if key == "handler" {
                handler = Some(settings.parse()?);
            } else {
                return Err(syn::Error::new(
                    key.span(),
                    "unknown field; expected `message` (and optional `handler`)",
                ));
            }

            // allow optional commas between fields (including trailing)
            if settings.peek(Token![,]) {
                settings.parse::<Token![,]>()?;
            }
        }

        let Some(message) = message else {
            return Err(syn::Error::new(pattern.span(), "topic missing `message`"));
        };

        topics.push(Topic {
            pattern,
            message,
            handler,
        });

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
    }
    Ok(topics)
}

pub fn topic_ident(topic: &LitStr) -> Ident {
    let s = topic.value();
    let mut out = String::with_capacity(s.len() + 3);
    out.push_str("on_");
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    if out.as_bytes().get(3).is_some_and(u8::is_ascii_digit) {
        // `on_123` isn't a valid identifier start after the prefix for some cases;
        // make it unambiguous.
        out.insert_str(3, "t_");
    }
    Ident::new(&out, Span::call_site())
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::*;

    #[test]
    fn parses_example_like_input_without_commas() {
        let input = quote!({
            owner: "at",
            provider: MyProvider,
            http: [
                "/jobs/detector": {
                    method: get,
                    request: DetectionRequest
                    reply: DetectionReply
                }
            ],
            messaging: [
                "realtime-r9k.v1": {
                    message: R9kMessage
                }
            ]
        });

        let parsed: Input = syn::parse2(input).expect("should parse");

        let http = parsed.http.expect("should have http");
        assert_eq!(http.len(), 1);
        assert_eq!(http[0].path.value(), "/jobs/detector");
        assert!(http[0].params.is_empty());

        let messaging = parsed.messaging.expect("should have messaging");
        assert_eq!(messaging.len(), 1);
        assert_eq!(messaging[0].pattern.value(), "realtime-r9k.v1");
    }

    #[test]
    fn parses_http_path_params() {
        let input = quote!({
            owner: "at",
            provider: MyProvider,
            http: [
                "/god-mode/set-trip/{vehicle_id}/{trip_id}": {
                    method: get,
                    request: SetTripRequest,
                    reply: SetTripReply,
                }
            ]
        });

        let parsed: Input = syn::parse2(input).expect("should parse");
        let http = parsed.http.expect("should have http");
        assert_eq!(http.len(), 1);
        assert_eq!(http[0].params.len(), 2);
        assert_eq!(http[0].params[0].to_string(), "vehicle_id");
        assert_eq!(http[0].params[1].to_string(), "trip_id");
    }
}
