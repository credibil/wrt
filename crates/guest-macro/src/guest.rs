mod http;
mod messaging;

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Error, Ident, LitStr, Path, Result, Token};

use self::http::{GeneratedHttp, Http};
use self::messaging::{GeneratedMessaging, Messaging};

pub struct Config {
    pub owner: LitStr,
    pub provider: Ident,
    pub http: Option<Http>,
    pub messaging: Option<Messaging>,
}

impl Parse for Config {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut owner: Option<LitStr> = None;
        let mut provider: Option<Ident> = None;
        let mut http: Option<Http> = None;
        let mut messaging: Option<Messaging> = None;

        let settings;
        syn::braced!(settings in input);
        let fields = Punctuated::<Opt, Token![,]>::parse_terminated(&settings)?;

        for field in fields.into_pairs() {
            match field.into_value() {
                Opt::Owner(o) => {
                    if owner.is_some() {
                        return Err(Error::new(o.span(), "cannot specify second owner"));
                    }
                    owner = Some(o);
                }
                Opt::Provider(p) => {
                    if provider.is_some() {
                        return Err(Error::new(p.span(), "cannot specify second provider"));
                    }
                    provider = Some(p);
                }
                Opt::Http(h) => {
                    http = Some(h);
                }
                Opt::Messaging(m) => {
                    messaging = Some(m);
                }
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

#[allow(clippy::large_enum_variant)]
enum Opt {
    Owner(syn::LitStr),
    Provider(Ident),
    Http(Http),
    Messaging(Messaging),
}

impl Parse for Opt {
    fn parse(input: ParseStream) -> Result<Self> {
        let l = input.lookahead1();
        if l.peek(kw::owner) {
            input.parse::<kw::owner>()?;
            input.parse::<Token![:]>()?;
            Ok(Self::Owner(input.parse::<LitStr>()?))
        } else if l.peek(kw::provider) {
            input.parse::<kw::provider>()?;
            input.parse::<Token![:]>()?;
            Ok(Self::Provider(input.parse::<Ident>()?))
        } else if l.peek(kw::http) {
            input.parse::<kw::http>()?;
            input.parse::<Token![:]>()?;
            let list;
            syn::bracketed!(list in input);
            Ok(Self::Http(list.parse()?))
        } else if l.peek(kw::messaging) {
            input.parse::<kw::messaging>()?;
            input.parse::<Token![:]>()?;
            let list;
            syn::bracketed!(list in input);
            Ok(Self::Messaging(list.parse()?))
        } else {
            Err(l.error())
        }
    }
}

mod kw {
    syn::custom_keyword!(owner);
    syn::custom_keyword!(provider);
    syn::custom_keyword!(http);
    syn::custom_keyword!(messaging);
}

struct Generated {
    pub owner: LitStr,
    pub provider: Ident,
    pub http: Option<GeneratedHttp>,
    pub messaging: Option<GeneratedMessaging>,
}

impl From<Config> for Generated {
    fn from(input: Config) -> Self {
        Self {
            owner: input.owner,
            provider: input.provider,
            http: input.http.map(GeneratedHttp::from),
            messaging: input.messaging.map(GeneratedMessaging::from),
        }
    }
}

pub fn expand(config: Config) -> TokenStream {
    let generated = Generated::from(config);

    let owner = generated.owner;
    let provider = generated.provider;
    let client = quote! {
        Client::new(#owner).provider(#provider::new())
    };

    let http_mod = generated.http.as_ref().map(|h| http::expand(h, &client));
    let messaging_mod = generated.messaging.as_ref().map(|m| messaging::expand(m, &client));

    quote! {
        #[cfg(target_arch = "wasm32")]
        mod __buildgen_guest {
            use anyhow::{Context, Result};
            use warp_sdk::api::Client;

            use super::*;

            #http_mod
            #messaging_mod
        }
    }
}

/// Derive a handler method name from the request type name
fn method_name(path: &Path) -> Ident {
    let Some(ident) = path.segments.last() else {
        return format_ident!("handle");
    };

    // get the first word of the last segment
    let ident_str = quote! {#ident}.to_string();
    let new_word =
        ident_str[1..].chars().position(char::is_uppercase).unwrap_or(ident_str.len() - 1);
    let method_name = &ident_str[0..=new_word];

    format_ident!("{method_name}")
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::*;

    #[test]
    fn method_from_path() {
        // one word
        let path = Path::from(format_ident!("Hello"));
        let name = method_name(&path);
        assert_eq!(name, format_ident!("Hello"));

        // two words
        let path = Path::from(format_ident!("HelloWorld"));
        let name = method_name(&path);
        assert_eq!(name, format_ident!("Hello"));
    }

    #[test]
    fn parse_config() {
        let input = quote!({
            owner: "at",
            provider: MyProvider,
            http: [
                "/jobs/detector": {
                    method: get,
                    request: DetectionRequest,
                    reply: DetectionReply
                }
            ],
            messaging: [
                "realtime-r9k.v1": {
                    message: R9kMessage
                }
            ]
        });

        let parsed: Config = syn::parse2(input).expect("should parse");

        let http = parsed.http.expect("should have http");
        assert_eq!(http.routes.len(), 1);
        assert_eq!(http.routes[0].path.value(), "/jobs/detector");
        assert!(http.routes[0].params.is_empty());

        let messaging = parsed.messaging.expect("should have messaging");
        assert_eq!(messaging.topics.len(), 1);
        assert_eq!(messaging.topics[0].pattern.value(), "realtime-r9k.v1");
    }

    #[test]
    fn parse_http_path_params() {
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

        let parsed: Config = syn::parse2(input).expect("should parse");
        let http = parsed.http.expect("should have http");
        assert_eq!(http.routes.len(), 1);
        assert_eq!(http.routes[0].params.len(), 2);
        assert_eq!(http.routes[0].params[0].to_string(), "vehicle_id");
        assert_eq!(http.routes[0].params[1].to_string(), "trip_id");
    }
}
