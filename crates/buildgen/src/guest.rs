pub mod expand;
pub mod generate;
pub mod http;
pub mod messaging;

use proc_macro2::Span;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Result, Token, Type};

use self::messaging::parse::Messaging;
use crate::guest::http::parse::{Http, Route};
use crate::guest::messaging::parse::Topic;

pub struct Input {
    pub owner: LitStr,
    pub provider: Type,
    pub http: Option<Http>,
    pub messaging: Option<Messaging>,
}

impl Parse for Input {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        syn::braced!(content in input);

        // let mut out = Self::default();
        let mut owner: Option<LitStr> = None;
        let mut provider: Option<Type> = None;
        let mut http: Option<Http> = None;
        let mut messaging: Option<Messaging> = None;

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
                    http = Some(list.parse()?);
                }
                "messaging" => {
                    let list;
                    syn::bracketed!(list in content);
                    messaging = Some(list.parse()?);
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

// pub fn topic_ident(topic: &LitStr) -> Ident {
//     let s = topic.value();
//     let mut out = String::with_capacity(s.len() + 3);
//     out.push_str("on_");
//     for ch in s.chars() {
//         if ch.is_ascii_alphanumeric() {
//             out.push(ch.to_ascii_lowercase());
//         } else {
//             out.push('_');
//         }
//     }
//     if out.as_bytes().get(3).is_some_and(u8::is_ascii_digit) {
//         // `on_123` isn't a valid identifier start after the prefix for some cases;
//         // make it unambiguous.
//         out.insert_str(3, "t_");
//     }
//     Ident::new(&out, Span::call_site())
// }

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
        assert_eq!(http.routes.len(), 1);
        assert_eq!(http.routes[0].path.value(), "/jobs/detector");
        assert!(http.routes[0].params.is_empty());

        let messaging = parsed.messaging.expect("should have messaging");
        assert_eq!(messaging.topics.len(), 1);
        assert_eq!(messaging.topics[0].pattern.value(), "realtime-r9k.v1");
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
        assert_eq!(http.routes.len(), 1);
        assert_eq!(http.routes[0].params.len(), 2);
        assert_eq!(http.routes[0].params[0].to_string(), "vehicle_id");
        assert_eq!(http.routes[0].params[1].to_string(), "trip_id");
    }
}
