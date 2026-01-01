//! # Generated Code Expansion
//!
//! Expands generated guest glue into a complete wasm32 guest implementation.

use proc_macro2::TokenStream;
use quote::quote;

use crate::guest::generate::Generated;
use crate::guest::{http, messaging};

pub fn expand(generated: Generated) -> TokenStream {
    let owner = generated.owner;
    let provider = generated.provider;
    let client = quote! {
        Client::new(#owner).provider(#provider::new())
    };

    let http_mod = generated.http.as_ref().map(|h| http::expand::expand_http(h, &client));
    let messaging_mod =
        generated.messaging.as_ref().map(|m| messaging::expand::expand_messaging(m, &client));

    quote! {
        #[cfg(target_arch = "wasm32")]
        mod __buildgen_guest {
            use anyhow::{Context, Result};
            use fabric::api::Client;

            use super::*;

            #http_mod
            #messaging_mod
        }
    }
}
