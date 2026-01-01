use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Result, Token, Type};

pub struct Messaging {
    pub topics: Vec<Topic>,
}

impl Parse for Messaging {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut topics = Vec::new();

        while !input.is_empty() {
            let topic: Topic = input.parse()?;
            topics.push(topic);

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(Self { topics })
    }
}

pub struct Topic {
    pub pattern: LitStr,
    pub message: Type,
}

impl Parse for Topic {
    fn parse(input: ParseStream) -> Result<Self> {
        let pattern: LitStr = input.parse()?;
        input.parse::<Token![:]>()?;

        let settings;
        syn::braced!(settings in input);

        let mut message: Option<Type> = None;

        while !settings.is_empty() {
            let key: Ident = settings.parse()?;
            settings.parse::<Token![:]>()?;

            if key == "message" {
                message = Some(settings.parse()?);
            } else {
                return Err(syn::Error::new(key.span(), "unknown field; expected `message`"));
            }

            // allow optional commas between fields (including trailing)
            if settings.peek(Token![,]) {
                settings.parse::<Token![,]>()?;
            }
        }

        let Some(message) = message else {
            return Err(syn::Error::new(pattern.span(), "topic missing `message`"));
        };

        Ok(Self { pattern, message })
    }
}

pub struct GeneratedMessaging {
    pub topics: Vec<GeneratedTopic>,
}

impl From<Messaging> for GeneratedMessaging {
    fn from(messaging: Messaging) -> Self {
        Self {
            topics: messaging.topics.into_iter().map(GeneratedTopic::from).collect(),
        }
    }
}

pub struct GeneratedTopic {
    pub pattern: LitStr,
    pub message_type: Type,
    pub handler_name: Ident,
}

impl From<Topic> for GeneratedTopic {
    fn from(topic: Topic) -> Self {
        let message = topic.message;
        let message_str = quote! {#message}.to_string();
        let handler_name =
            message_str.strip_suffix("Message").unwrap_or(&message_str).to_lowercase();

        Self {
            pattern: topic.pattern,
            message_type: message,
            handler_name: format_ident!("{handler_name}"),
        }
    }
}

pub fn expand(messaging: &GeneratedMessaging, client: &TokenStream) -> TokenStream {
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

fn expand_topic(topic: &GeneratedTopic) -> TokenStream {
    let pattern = &topic.pattern;
    let handler_name = &topic.handler_name;

    quote! {
        t if t.contains(#pattern) => #handler_name(&message.data()).await,
    }
}

fn expand_processor(topic: &GeneratedTopic, client: &TokenStream) -> TokenStream {
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
