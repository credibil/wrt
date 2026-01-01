use quote::{format_ident, quote};
use syn::{Ident, LitStr, Type};

use crate::guest as parsed;

pub struct MessagingGuest {
    pub topics: Vec<Topic>,
}

pub struct Topic {
    pub pattern: LitStr,
    pub message_type: Type,
    pub handler_name: Ident,
}

pub fn generate(messaging: parsed::Messaging) -> MessagingGuest {
    MessagingGuest {
        topics: messaging.topics.into_iter().map(generate_topic).collect(),
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
