use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, LitStr, Path, Type};

use crate::guest::{self as parsed, Input, topic_ident};

pub struct Generated {
    pub http: Option<HttpGuest>,
    pub messaging: Option<MessagingGuest>,
}

pub struct HttpGuest {
    pub routes: Vec<Route>,
}

pub struct Route {
    pub path: LitStr,
    pub method: Ident,
    pub handler: TokenStream,
    pub request: Option<Type>,
    pub response: Option<Type>,
}

pub struct MessagingGuest {
    pub topics: Vec<Topic>,
}

pub struct Topic {
    pub pattern: LitStr,
    pub message_type: Type,
    pub handler: TokenStream,
}

impl TryFrom<Input> for Generated {
    type Error = syn::Error;

    fn try_from(input: Input) -> Result<Self, Self::Error> {
        let http = if input.http.is_empty() {
            None
        } else {
            Some(HttpGuest {
                routes: input
                    .http
                    .into_iter()
                    .map(generate_route)
                    .collect::<Result<Vec<_>, _>>()?,
            })
        };

        let messaging = if input.messaging.is_empty() {
            None
        } else {
            Some(MessagingGuest {
                topics: input.messaging.into_iter().map(generate_topic).collect(),
            })
        };

        Ok(Self { http, messaging })
    }
}

fn generate_route(route: parsed::Route) -> syn::Result<Route> {
    let parsed::Route {
        path,
        method,
        request,
        handler,
        response,
    } = route;

    let method_str = method.to_string().to_ascii_lowercase();
    let method_ident = match method_str.as_str() {
        "get" | "post" | "put" | "delete" | "patch" | "head" | "options" | "trace" | "connect" => {
            format_ident!("{method_str}")
        }
        _ => {
            return Err(syn::Error::new(
                method.span(),
                "unknown http method; expected one of: get, post, put, delete, patch, head, options",
            ));
        }
    };

    Ok(Route {
        path,
        method: method_ident,
        handler: qualify_path(&handler),
        request,
        response,
    })
}

fn generate_topic(topic: parsed::Topic) -> Topic {
    let parsed::Topic {
        pattern,
        message,
        handler,
    } = topic;

    let tokens = handler.as_ref().map_or_else(
        || {
            let ident = topic_ident(&pattern);
            quote!(super::#ident)
        },
        qualify_path,
    );

    Topic {
        pattern,
        message_type: message,
        handler: tokens,
    }
}

// If the user wrote just `foo`, they almost certainly mean "a sibling item
// where the macro is invoked". Since we generate inside a nested module, use
// `super::foo`.
fn qualify_path(path: &Path) -> TokenStream {
    if path.leading_colon.is_none() && path.segments.len() == 1 {
        let ident = &path.segments[0].ident;
        quote!(super::#ident)
    } else {
        quote!(#path)
    }
}
