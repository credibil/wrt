//! # Generated Code Expansion
//!
//! Expands generated guest glue into a complete wasm32 guest implementation.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::guest::generate::{Generated, HttpGuest, MessagingGuest, Route, Topic};

pub fn expand(generated: Generated) -> TokenStream {
    let owner = generated.owner;
    let provider = generated.provider;
    let client = quote! {
        Client::new(#owner).provider(#provider::new())
    };

    let http_guest = generated.http.as_ref().map(|http| expand_http(http, &client));
    let messaging_guest = generated.messaging.map(|messaging| expand_messaging(messaging, &client));

    quote! {
        #[cfg(target_arch = "wasm32")]
        mod __buildgen_guest {
            use super::*;

            #http_guest
            #messaging_guest
        }
    }
}

fn expand_http(http: &HttpGuest, client: &TokenStream) -> TokenStream {
    let routes = http.routes.iter().map(expand_route);
    let handlers = http.routes.iter().map(|r| expand_handler(r, client));

    quote! {
        mod http {
            use anyhow::{Context, Result};
            use fabric::api::{Client, HttpResult, Reply};

            use super::*;

            pub struct Http;
            wasip3::http::proxy::export!(Http);

            impl wasip3::exports::http::handler::Guest for Http {
                async fn handle(
                    request: wasip3::http::types::Request,
                ) -> core::result::Result<wasip3::http::types::Response, wasip3::http::types::ErrorCode> {
                    let router = axum::Router::new()
                        #(#routes)*;
                    wasi_http::serve(router, request).await
                }
            }

            #(#handlers)*
        }
    }
}

fn expand_route(route: &Route) -> TokenStream {
    let path = &route.path;
    let method = &route.method;
    let handler_name = &route.handler_name;

    quote! {
        .route(#path, axum::routing::#method(#handler_name))
    }
}

fn expand_handler(route: &Route, client: &TokenStream) -> TokenStream {
    let handler_name = &route.handler_name;
    let request = &route.request;
    let reply = &route.reply;

    let empty = &Vec::<Ident>::new();
    let params = route.params.as_ref().unwrap_or(empty);

    // generate handler function name and signature
    let handler_fn = if route.method == "get" {
        let param_args = if params.is_empty() {
            quote! {}
        } else if params.len() == 1 {
            quote! { axum::extract::Path(#(#params),*): axum::extract::Path<String> }
        } else {
            let mut param_types = Vec::new();
            for _ in 0..params.len() {
                param_types.push(format_ident!("String"));
            }

            quote! { axum::extract::Path((#(#params),*)): axum::extract::Path<(#(#param_types),*)> }
        };

        quote! { #handler_name(#param_args) }
    } else {
        quote! { #handler_name(body: bytes::Bytes) }
    };

    // generate request parameter and type
    let request = if route.method == "get" {
        if params.is_empty() {
            quote! { #request }
        } else if params.len() == 1 {
            quote! { #request::try_from(#(#params),*).context("parsing request")? }
        } else {
            quote! { #request::try_from((#(#params),*)).context("parsing request")? }
        }
    } else {
        quote! { #request::try_from(body.as_ref()).context("parsing request")? }
    };

    quote! {
        async fn #handler_fn -> HttpResult<Reply<#reply>> {
            let client = #client;
            let request = #request;
            let reply = client.request(request).await.context("processing request")?;
            Ok(reply)
        }
    }
}

fn expand_messaging(messaging: MessagingGuest, _client: &TokenStream) -> TokenStream {
    let arms = messaging.topics.into_iter().map(expand_topic);

    quote! {
        mod messaging {
            use super::*;

            pub struct Messaging;
            wasi_messaging::export!(Messaging with_types_in wasi_messaging);

            impl wasi_messaging::incoming_handler::Guest for Messaging {
                async fn handle(
                    message: wasi_messaging::types::Message,
                ) -> core::result::Result<(), wasi_messaging::types::Error> {
                    let topic = message.topic().unwrap_or_default();
                    match topic.as_str() {
                        #(#arms)*
                        _ => Ok(()),
                    }
                }
            }
        }
    }
}

fn expand_topic(topic: Topic) -> TokenStream {
    let Topic {
        pattern,
        message_type,
        handler,
    } = topic;

    quote! {
        #pattern => {
            let payload = message.data();
            let parsed: #message_type =
                <#message_type as ::core::convert::TryFrom<&[u8]>>::try_from(payload.as_slice())
                    .map_err(|e| wasi_messaging::types::Error::Other(format!("{e}")))?;
            #handler(parsed)
                .await
                .map_err(|e| wasi_messaging::types::Error::Other(e.to_string()))?;
            Ok(())
        }
    }
}
