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

    let http_mod = generated.http.as_ref().map(|h| expand_http(h, &client));
    let messaging_mod = generated.messaging.map(|m| expand_messaging(&m, &client));

    quote! {
        #[cfg(target_arch = "wasm32")]
        mod __buildgen_guest {
            use anyhow::{Context, Result};
            use fabric::api::Client;

            use super::*;

            #http_mod
            #messaging_mod
        }
    }
}

fn expand_http(http: &HttpGuest, client: &TokenStream) -> TokenStream {
    let routes = http.routes.iter().map(expand_route);
    let handlers = http.routes.iter().map(|r| expand_handler(r, client));

    quote! {
        mod http {
            use fabric::api::{HttpResult, Reply};

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

fn expand_messaging(messaging: &MessagingGuest, client: &TokenStream) -> TokenStream {
    let topic_arms = messaging.topics.iter().map(expand_topic);
    let processors = messaging.topics.iter().map(|t| expand_processor(t, client));

    quote! {
        mod messaging {
            use wasi_messaging::types::Message;

            use super::*;

            pub struct Messaging;
            wasi_messaging::export!(Messaging with_types_in wasi_messaging);

            impl wasi_messaging::incoming_handler::Guest for Messaging {
                async fn handle(
                    message: wasi_messaging::types::Message,
                ) -> core::result::Result<(), wasi_messaging::types::Error> {
                    let topic = message.topic().unwrap_or_default();

                    // check we're processing topics for the correct environment
                    // let env = &Provider::new().config.environment;
                    let env = std::env::var("ENV").unwrap_or_default();
                    let Some(topic) = topic.strip_prefix(&format!("{env}-")) else {
                        return Err(wasi_messaging::types::Error::Other("Incorrect environment".to_string()));
                    };

                    if let Err(e) = match &topic {
                        #(#topic_arms)*
                        _ => return Err(wasi_messaging::types::Error::Other("Unhandled topic".to_string())),
                    } {
                        return Err(wasi_messaging::types::Error::Other(e.to_string()));
                    }

                    Ok(())
                }
            }

            #(#processors)*
        }
    }
}

fn expand_topic(topic: &Topic) -> TokenStream {
    let pattern = &topic.pattern;
    let handler_name = &topic.handler_name;

    quote! {
        t if t.contains(#pattern) => #handler_name(&message.data()).await,
    }
}

fn expand_processor(topic: &Topic, client: &TokenStream) -> TokenStream {
    let handler_fn = &topic.handler_name;
    let message = &topic.message_type;

    quote! {
        async fn #handler_fn(message: &[u8]) -> Result<()> {
            let client = #client;
            let request = #message::try_from(message).context("parsing message")?;
            client.request(request).await?;
            Ok(())
        }
    }
}
