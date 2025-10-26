//! # OpenTelemetry Attribute Macros

use proc_macro::TokenStream;
use quote::quote;
use syn::meta::{self, ParseNestedMeta};
use syn::parse::Result;
use syn::{Expr, ItemFn, LitStr, parse_macro_input};

/// Instruments a function using the `[wasi_otel::instrument]` function.
///
/// This macro can be used to automatically create spans for functions, making
/// it easier to add observability to your code.
#[proc_macro_attribute]
pub fn instrument(args: TokenStream, item: TokenStream) -> TokenStream {
    // macro's attributes
    let mut attrs = Attributes::default();
    let arg_parser = meta::parser(|meta| attrs.parse(&meta));
    parse_macro_input!(args with arg_parser);

    let item_fn = parse_macro_input!(item as ItemFn);
    let signature = signature(&item_fn);
    let body = body(attrs, &item_fn);

    // println!("signature: {signature}");
    // println!("body: {body}");

    // recreate function with the instrument macro wrapping the body
    let new_fn = quote! {
        #signature {
            let _guard = ::tracing::Span::current().is_none().then(::wasi_otel::init);
            #body
        }
    };

    TokenStream::from(new_fn)
}

fn signature(item_fn: &ItemFn) -> proc_macro2::TokenStream {
    let signature = item_fn.sig.clone();

    if item_fn.sig.asyncness.is_some() {
        quote! {
            #[allow(clippy::future_not_send)]
            #signature
        }
    } else {
        quote! {#signature}
    }
}

fn body(attrs: Attributes, item_fn: &ItemFn) -> proc_macro2::TokenStream {
    let name = item_fn.sig.ident.clone();
    let block = item_fn.block.clone();

    let span_name = attrs.name.unwrap_or_else(|| LitStr::new(&name.to_string(), name.span()));
    let level =
        attrs.level.map_or_else(|| quote! { ::tracing::Level::INFO }, |level| quote! {#level});

    // `instrument` async functions
    if item_fn.sig.asyncness.is_some() {
        quote! {
            ::tracing::Instrument::instrument(
                #block,
                ::tracing::span!(#level, #span_name)
            ).into_inner()
        }
    } else {
        quote! {
            ::tracing::span!(#level, #span_name).in_scope(|| {
                #block
            })
        }
    }
}

#[derive(Default)]
struct Attributes {
    name: Option<LitStr>,
    level: Option<Expr>,
}

// See https://docs.rs/syn/latest/syn/meta/fn.parser.html
impl Attributes {
    fn parse(&mut self, meta: &ParseNestedMeta) -> Result<()> {
        if meta.path.is_ident("name") {
            self.name = Some(meta.value()?.parse()?);
            Ok(())
        } else if meta.path.is_ident("level") {
            self.level = Some(meta.value()?.parse()?);
            Ok(())
        } else {
            Err(meta.error("unsupported property"))
        }
    }
}
