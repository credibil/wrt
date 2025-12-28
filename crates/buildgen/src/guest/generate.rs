use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use crate::guest::GuestInput;

pub struct Generated {
    pub http: Option<TokenStream>,
    pub messaging: Option<TokenStream>,
}

impl TryFrom<GuestInput> for Generated {
    type Error = syn::Error;

    fn try_from(input: GuestInput) -> Result<Self, Self::Error> {
        let http = input.http.as_ref().map(|config| {
            let routes = config.routes.iter().map(|route| {
                let path = &route.path;
                let method = &route.method;
                let handler = &route.handler;
                quote! {
                    .route(#path, wasi_http::axum::routing::#method(#handler))
                }
            });

            quote! {
                pub struct Http;
                wasip3::http::proxy::export!(Http);

                impl wasip3::exports::http::handler::Guest for Http {
                    /// Routes incoming HTTP requests to handlers.
                    async fn handle(request: wasip3::http::types::Request) -> wasi_http::Result<wasip3::http::types::Response, wasip3::http::types::ErrorCode> {
                        let router = wasi_http::axum::Router::new()
                            #(#routes)*;
                        wasi_http::serve(router, request).await
                    }
                }
            }
        });

        let messaging = input.messaging.as_ref().map(|config| {
            let matches = config.handlers.iter().map(|handler| {
                let topic = &handler.topic;
                let message_type = &handler.message;
                
                // For messaging, if no handler is specified in the DSL, 
                // we assume a handler function named after the message type (snake_case).
                let handler_name = field_ident(message_type);
                
                quote! {
                    #topic => {
                        let msg: #message_type = serde_json::from_slice(&message.data())
                            .map_err(|e| wasi_messaging::types::Error::Other(e.to_string()))?;
                        #handler_name(msg).await
                            .map_err(|e| wasi_messaging::types::Error::Other(e.to_string()))?;
                    }
                }
            });

            quote! {
                pub struct Messaging;
                wasi_messaging::export!(Messaging with_types_in wasi_messaging);

                impl wasi_messaging::incoming_handler::Guest for Messaging {
                    /// Handles incoming messages from subscribed topics.
                    async fn handle(message: wasi_messaging::types::Message) -> anyhow::Result<(), wasi_messaging::types::Error> {
                        let topic = message.topic().unwrap_or_default();
                        match topic.as_str() {
                            #(#matches)*
                            _ => {
                                tracing::warn!("received message for unknown topic: {}", topic);
                            }
                        }
                        Ok(())
                    }
                }
            }
        });

        Ok(Self { http, messaging })
    }
}

/// Generates a field name for a type (e.g. for a handler function).
fn field_ident(field_type: &syn::Type) -> syn::Ident {
    let type_str = quote! {#field_type}.to_string();

    // convert the type string to a snake_case
    let mut field_str = String::new();
    for char in type_str.chars() {
        if char.is_uppercase() {
            if !field_str.is_empty() {
                field_str.push('_');
            }
            field_str.push_str(&char.to_lowercase().to_string());
        } else {
            field_str.push(char);
        }
    }

    format_ident!("{field_str}")
}
