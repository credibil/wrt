use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Ident, LitStr, Path, Result, Token};

use crate::guest::method_name;

pub struct Messaging {
    pub topics: Vec<Topic>,
}

impl Parse for Messaging {
    fn parse(input: ParseStream) -> Result<Self> {
        let topics = Punctuated::<Topic, Token![,]>::parse_terminated(input)?;
        Ok(Self {
            topics: topics.into_iter().collect(),
        })
    }
}

pub struct Topic {
    pub pattern: LitStr,
    pub message: Path,
}

impl Parse for Topic {
    fn parse(input: ParseStream) -> Result<Self> {
        let pattern: LitStr = input.parse()?;
        input.parse::<Token![:]>()?;

        let mut message: Option<Path> = None;

        let settings;
        syn::braced!(settings in input);
        let fields = Punctuated::<Opt, Token![,]>::parse_terminated(&settings)?;

        for field in fields.into_pairs() {
            match field.into_value() {
                Opt::Message(m) => {
                    if message.is_some() {
                        return Err(syn::Error::new(m.span(), "cannot specify second message"));
                    }
                    message = Some(m);
                }
            }
        }

        let Some(message) = message else {
            return Err(syn::Error::new(pattern.span(), "topic missing `message`"));
        };

        Ok(Self { pattern, message })
    }
}

mod kw {
    syn::custom_keyword!(message);
}

enum Opt {
    Message(Path),
}

impl Parse for Opt {
    fn parse(input: ParseStream) -> Result<Self> {
        let l = input.lookahead1();
        if l.peek(kw::message) {
            input.parse::<kw::message>()?;
            input.parse::<Token![:]>()?;
            Ok(Self::Message(input.parse::<Path>()?))
        } else {
            Err(l.error())
        }
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
    pub message: Path,
    pub handler_name: Ident,
}

impl From<Topic> for GeneratedTopic {
    fn from(topic: Topic) -> Self {
        let handler_name = method_name(&topic.message);

        Self {
            pattern: topic.pattern,
            message: topic.message,
            handler_name,
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
    let message = &topic.message;

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
