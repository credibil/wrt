//! # Generated Code Expansion
//!
//! Expands generated guest glue into a complete wasm32 guest implementation.

use proc_macro2::TokenStream;
use quote::quote;

use crate::guest::generate::{Generated, Route, Topic};

pub fn expand(generated: Generated) -> TokenStream {
    let http_mod = generated.http.map(expand_http);
    let messaging_mod = generated.messaging.map(expand_messaging);

    quote! {
        #[cfg(target_arch = "wasm32")]
        mod __buildgen_guest {
            use super::*;

            #http_mod
            #messaging_mod
        }
    }
}

fn expand_http(http: crate::guest::generate::HttpGuest) -> TokenStream {
    let routes = http.routes.into_iter().map(expand_route);

    quote! {
        mod http {
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
        }
    }
}

fn expand_route(route: Route) -> TokenStream {
    let Route {
        path,
        method,
        handler,
        ..
    } = route;

    quote! {
        .route(#path, axum::routing::#method(#handler))
    }
}

fn expand_messaging(messaging: crate::guest::generate::MessagingGuest) -> TokenStream {
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
