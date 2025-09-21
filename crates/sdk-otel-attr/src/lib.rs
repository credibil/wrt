//! # OpenTelemetry Attribute Macros

use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::meta::{self, ParseNestedMeta};
use syn::parse::Result;
use syn::spanned::Spanned;
use syn::{Expr, ItemFn, LitStr, ReturnType, parse_macro_input, parse_quote};

/// Instruments a function using the `[sdk_otel::instrument]` function.
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

    // recreate function with the instrument macro wrapping the body
    let new_fn = quote! {
        #signature {
            let _guard = if ::tracing::Span::current().is_none() {
                let shutdown = ::sdk_otel::init();
                Some(shutdown)
            } else {
                None
            };
            #body
        }
    };

    TokenStream::from(new_fn)
}

// Function signature
fn signature(item_fn: &ItemFn) -> proc_macro2::TokenStream {
    let ident = item_fn.sig.ident.clone();
    let inputs = item_fn.sig.inputs.clone();
    let output = item_fn.sig.output.clone();

    // rewrite async functions to return `Future + Send`
    if item_fn.sig.asyncness.is_some() {
        let (return_type, return_span) = if let ReturnType::Type(_, return_type) = &output {
            (return_type.to_owned(), return_type.span())
        } else {
            (parse_quote! { () }, ident.span())
        };
        quote_spanned! {return_span=>
            #[allow(refining_impl_trait)]
            fn #ident(#inputs) -> impl Future<Output = #return_type> + Send
        }
    } else {
        quote! {fn #ident(#inputs) #output}
    }
}

// Function body
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
                async move {
                    #block
                },
                ::tracing::span!(#level, #span_name)
            )
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
