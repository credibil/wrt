use proc_macro2::TokenStream;
use quote::quote;

use crate::guest::generate::Generated;

pub fn expand(generated: Generated) -> TokenStream {
    let Generated { http, messaging } = generated;

    quote! {
        #http
        #messaging
    }
}
