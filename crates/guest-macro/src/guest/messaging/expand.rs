use proc_macro2::TokenStream;
use quote::quote;

use crate::guest::messaging::generate::{Messaging, Topic};

pub fn expand(messaging: &Messaging, client: &TokenStream) -> TokenStream {
    let topic_arms = messaging.topics.iter().map(expand_topic);
    let processors = messaging.topics.iter().map(|t| expand_processor(t, client));

    quote! {
        mod messaging {
            use wasi_messaging::types::Message;

            use super::*;

            pub struct Messaging;
            wasi_messaging::export!(Messaging with_types_in wasi_messaging);

            impl wasi_messaging::incoming_handler::Guest for Messaging {
                #[wasi_otel::instrument]
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
        #[wasi_otel::instrument]
        async fn #handler_fn(message: &[u8]) -> Result<()> {
            let client = #client;
            let request = #message::try_from(message).context("parsing message")?;
            client.request(request).await?;
            Ok(())
        }
    }
}
