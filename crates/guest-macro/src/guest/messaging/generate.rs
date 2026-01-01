use quote::{format_ident, quote};
use syn::{Ident, LitStr, Type};

use crate::guest::messaging::parse;

pub struct Messaging {
    pub topics: Vec<Topic>,
}

impl From<parse::Messaging> for Messaging {
    fn from(messaging: parse::Messaging) -> Self {
        Self {
            topics: messaging.topics.into_iter().map(Topic::from).collect(),
        }
    }
}

pub struct Topic {
    pub pattern: LitStr,
    pub message_type: Type,
    pub handler_name: Ident,
}

impl From<parse::Topic> for Topic {
    fn from(topic: parse::Topic) -> Self {
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
