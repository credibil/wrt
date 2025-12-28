pub mod expand;
pub mod generate;

use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Token, Type, braced, bracketed};

mod kw {
    syn::custom_keyword!(http);
    syn::custom_keyword!(messaging);
    syn::custom_keyword!(method);
    syn::custom_keyword!(request);
    syn::custom_keyword!(handler);
    syn::custom_keyword!(response);
    syn::custom_keyword!(message);
}

pub struct GuestInput {
    pub http: Option<HttpConfig>,
    pub messaging: Option<MessagingConfig>,
}

pub struct HttpConfig {
    pub routes: Vec<HttpRoute>,
}

pub struct HttpRoute {
    pub path: LitStr,
    pub method: Ident,
    pub _request: Type,
    pub handler: Ident,
    pub _response: Type,
}

pub struct MessagingConfig {
    pub handlers: Vec<MessagingHandler>,
}

pub struct MessagingHandler {
    pub topic: LitStr,
    pub message: Type,
}

impl Parse for GuestInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        braced!(content in input);

        let mut http = None;
        let mut messaging = None;

        while !content.is_empty() {
            let lookahead = content.lookahead1();
            if lookahead.peek(kw::http) {
                content.parse::<kw::http>()?;
                content.parse::<Token![:]>()?;
                http = Some(content.parse()?);
            } else if lookahead.peek(kw::messaging) {
                content.parse::<kw::messaging>()?;
                content.parse::<Token![:]>()?;
                messaging = Some(content.parse()?);
            } else {
                return Err(lookahead.error());
            }

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(Self { http, messaging })
    }
}

impl Parse for HttpConfig {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        bracketed!(content in input);
        let mut routes = Vec::new();
        while !content.is_empty() {
            routes.push(content.parse()?);
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }
        Ok(Self { routes })
    }
}

impl Parse for HttpRoute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: LitStr = input.parse()?;
        input.parse::<Token![:]>()?;
        let content;
        braced!(content in input);

        let mut method = None;
        let mut request = None;
        let mut handler = None;
        let mut response = None;

        while !content.is_empty() {
            let lookahead = content.lookahead1();
            if lookahead.peek(kw::method) {
                content.parse::<kw::method>()?;
                content.parse::<Token![:]>()?;
                method = Some(content.parse::<Ident>()?);
            } else if lookahead.peek(kw::request) {
                content.parse::<kw::request>()?;
                content.parse::<Token![:]>()?;
                request = Some(content.parse::<Type>()?);
            } else if lookahead.peek(kw::handler) {
                content.parse::<kw::handler>()?;
                content.parse::<Token![:]>()?;
                handler = Some(content.parse::<Ident>()?);
            } else if lookahead.peek(kw::response) {
                content.parse::<kw::response>()?;
                content.parse::<Token![:]>()?;
                response = Some(content.parse::<Type>()?);
            } else {
                return Err(lookahead.error());
            }

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            path,
            method: method.ok_or_else(|| content.error("missing method"))?,
            _request: request.ok_or_else(|| content.error("missing request"))?,
            handler: handler.ok_or_else(|| content.error("missing handler"))?,
            _response: response.ok_or_else(|| content.error("missing response"))?,
        })
    }
}

impl Parse for MessagingConfig {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        bracketed!(content in input);
        let mut handlers = Vec::new();
        while !content.is_empty() {
            handlers.push(content.parse()?);
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }
        Ok(Self { handlers })
    }
}

impl Parse for MessagingHandler {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let topic: LitStr = input.parse()?;
        input.parse::<Token![:]>()?;
        let content;
        braced!(content in input);

        let mut message = None;

        while !content.is_empty() {
            let lookahead = content.lookahead1();
            if lookahead.peek(kw::message) {
                content.parse::<kw::message>()?;
                content.parse::<Token![:]>()?;
                message = Some(content.parse::<Type>()?);
            } else {
                return Err(lookahead.error());
            }

            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(Self {
            topic,
            message: message.ok_or_else(|| content.error("missing message"))?,
        })
    }
}
