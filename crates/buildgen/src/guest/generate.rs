use quote::{format_ident, quote};
use syn::{Ident, LitStr, Type};

use crate::guest::{self as parsed, Input};

pub struct Generated {
    pub owner: LitStr,
    pub provider: Type,
    pub http: Option<HttpGuest>,
    pub messaging: Option<MessagingGuest>,
}

pub struct HttpGuest {
    pub routes: Vec<Route>,
}

pub struct Route {
    pub path: LitStr,
    #[allow(dead_code)]
    pub params: Option<Vec<Ident>>,
    pub method: Ident,
    pub handler_name: Ident,
    pub request: Type,
    pub reply: Type,
}

pub struct MessagingGuest {
    pub topics: Vec<Topic>,
}

pub struct Topic {
    pub pattern: LitStr,
    pub message_type: Type,
    pub handler_name: Ident,
}

impl TryFrom<Input> for Generated {
    type Error = syn::Error;

    fn try_from(input: Input) -> Result<Self, Self::Error> {
        let http = input.http.map(|http| HttpGuest {
            routes: http.into_iter().map(generate_route).collect(),
        });
        let messaging = input.messaging.map(|messaging| MessagingGuest {
            topics: messaging.into_iter().map(generate_topic).collect(),
        });

        Ok(Self {
            owner: input.owner,
            provider: input.provider,
            http,
            messaging,
        })
    }
}

fn generate_route(route: parsed::Route) -> Route {
    let request = route.request;
    let request_str = quote! {#request}.to_string();
    let handler_name = request_str.strip_suffix("Request").unwrap_or(&request_str).to_lowercase();

    Route {
        path: route.path,
        params: Some(route.params),
        method: route.method,
        handler_name: format_ident!("{handler_name}"),
        request,
        reply: route.reply,
    }
}

fn generate_topic(topic: parsed::Topic) -> Topic {
    let message = topic.message;
    let message_str = quote! {#message}.to_string();
    let handler_name = message_str.strip_suffix("Message").unwrap_or(&message_str).to_lowercase();

    Topic {
        pattern: topic.pattern,
        message_type: message,
        handler_name: format_ident!("{handler_name}"),
    }
}

// // If the user wrote just `foo`, they almost certainly mean "a sibling item
// // where the macro is invoked". Since we generate inside a nested module, use
// // `super::foo`.
// fn qualify_path(path: &Path) -> TokenStream {
//     if path.leading_colon.is_none() && path.segments.len() == 1 {
//         let ident = &path.segments[0].ident;
//         quote!(super::#ident)
//     } else {
//         quote!(#path)
//     }
// }
