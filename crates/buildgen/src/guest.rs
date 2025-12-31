pub mod expand;
pub mod generate;

use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Path, Result, Token, Type};

/// Configuration for the `guest!` macro.
///
/// Parses input in the form:
/// ```ignore
/// {
///   http: [
///     "/path": { method: get, handler: my_handler, request: MyReq, response: MyResp }
///   ],
///   messaging: [
///     "topic": { message: MyMessage, handler: on_topic }
///   ]
/// }
/// ```
#[derive(Default)]
pub struct Input {
    pub http: Vec<Route>,
    pub messaging: Vec<Topic>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        syn::braced!(content in input);

        let mut out = Self::default();

        while !content.is_empty() {
            let section: Ident = content.parse()?;
            content.parse::<Token![:]>()?;

            if section == "http" {
                let list;
                syn::bracketed!(list in content);
                out.http = parse_routes(&list)?;
            } else if section == "messaging" {
                let list;
                syn::bracketed!(list in content);
                out.messaging = parse_topics(&list)?;
            } else {
                return Err(syn::Error::new(
                    section.span(),
                    "unknown section; expected `http` or `messaging`",
                ));
            }

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(out)
    }
}

pub struct Route {
    pub path: LitStr,
    pub method: Ident,
    pub request: Option<Type>,
    pub handler: Path,
    pub response: Option<Type>,
}

pub struct Topic {
    pub pattern: LitStr,
    pub message: Type,
    pub handler: Option<Path>,
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
        let mut handler: Option<Path> = None;
        let mut response: Option<Type> = None;

        while !settings.is_empty() {
            let key: Ident = settings.parse()?;
            settings.parse::<Token![:]>()?;

            if key == "method" {
                method = Some(settings.parse()?);
            } else if key == "request" {
                request = Some(settings.parse()?);
            } else if key == "handler" {
                handler = Some(settings.parse()?);
            } else if key == "response" {
                response = Some(settings.parse()?);
            } else {
                return Err(syn::Error::new(
                    key.span(),
                    "unknown http field; expected `method`, `request`, `handler`, or `response`",
                ));
            }

            // allow optional commas between fields (including trailing)
            if settings.peek(Token![,]) {
                settings.parse::<Token![,]>()?;
            }
        }

        let Some(method) = method else {
            return Err(syn::Error::new(path.span(), "http route missing `method`"));
        };
        let Some(handler) = handler else {
            return Err(syn::Error::new(path.span(), "http route missing `handler`"));
        };

        routes.push(Route {
            path,
            method,
            request,
            handler,
            response,
        });

        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        }
    }
    Ok(routes)
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
            http: [
                "/jobs/detector": {
                    method: get,
                    request: String
                    handler: detection_request
                    response: DetectionResponse
                }
            ],
            messaging: [
                "realtime-r9k.v1": {
                    message: R9kMessage
                }
            ]
        });

        let parsed: Input = syn::parse2(input).expect("should parse");
        assert_eq!(parsed.http.len(), 1);
        assert_eq!(parsed.messaging.len(), 1);
        assert_eq!(parsed.http[0].path.value(), "/jobs/detector");
        assert_eq!(parsed.messaging[0].pattern.value(), "realtime-r9k.v1");
    }
}
