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
