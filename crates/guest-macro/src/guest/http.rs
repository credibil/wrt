use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Ident, LitStr, Path, PathSegment, Result, Token, parse_str};

pub struct Http {
    pub routes: Vec<Route>,
}

impl Parse for Http {
    fn parse(input: ParseStream) -> Result<Self> {
        let entries = Punctuated::<Route, Token![,]>::parse_terminated(input)?;
        let routes = entries.into_iter().collect();
        Ok(Self { routes })
    }
}

pub struct Route {
    pub path: LitStr,
    pub params: Vec<Ident>,
    pub method: Ident,
    pub request: Path,
    pub reply: Path,
}

impl Parse for Route {
    fn parse(input: ParseStream) -> Result<Self> {
        let path: LitStr = input.parse()?;
        input.parse::<Token![:]>()?;

        let mut method: Option<Ident> = None;
        let mut request: Option<Path> = None;
        let mut reply: Option<Path> = None;

        let settings;
        syn::braced!(settings in input);
        let fields = Punctuated::<Opt, Token![,]>::parse_terminated(&settings)?;

        for field in fields.into_pairs() {
            match field.into_value() {
                Opt::Method(m) => {
                    if method.is_some() {
                        return Err(syn::Error::new(m.span(), "cannot specify second method"));
                    }
                    method = Some(m);
                }
                Opt::Request(r) => {
                    if request.is_some() {
                        return Err(syn::Error::new(r.span(), "cannot specify second request"));
                    }
                    request = Some(r);
                }
                Opt::Reply(r) => {
                    if reply.is_some() {
                        return Err(syn::Error::new(r.span(), "cannot specify second reply"));
                    }
                    reply = Some(r);
                }
            }
        }

        // validate required fields
        let method = if let Some(method) = method {
            let method_str = method.to_string().to_lowercase();
            match method_str.as_str() {
                "get" | "post" => format_ident!("{method_str}"),
                _ => {
                    return Err(syn::Error::new(
                        method.span(),
                        "unsupported http method; expected `get` or `post`",
                    ));
                }
            }
        } else {
            format_ident!("get")
        };

        let Some(request) = request else {
            return Err(syn::Error::new(path.span(), "route is missing `request`"));
        };
        let Some(reply) = reply else {
            return Err(syn::Error::new(path.span(), "route is missing `reply`"));
        };
        let params = parse_params(&path)?;

        Ok(Self {
            path,
            params,
            method,
            request,
            reply,
        })
    }
}

mod kw {
    syn::custom_keyword!(method);
    syn::custom_keyword!(request);
    syn::custom_keyword!(reply);
}

enum Opt {
    Method(Ident),
    Request(Path),
    Reply(Path),
}

impl Parse for Opt {
    fn parse(input: ParseStream) -> Result<Self> {
        let l = input.lookahead1();
        if l.peek(kw::method) {
            input.parse::<kw::method>()?;
            input.parse::<Token![:]>()?;
            Ok(Self::Method(input.parse::<Ident>()?))
        } else if l.peek(kw::request) {
            input.parse::<kw::request>()?;
            input.parse::<Token![:]>()?;
            Ok(Self::Request(input.parse::<Path>()?))
        } else if l.peek(kw::reply) {
            input.parse::<kw::reply>()?;
            input.parse::<Token![:]>()?;
            Ok(Self::Reply(input.parse::<Path>()?))
        } else {
            Err(l.error())
        }
    }
}

fn parse_params(path: &LitStr) -> Result<Vec<Ident>> {
    let s = path.value();
    let mut params = Vec::<Ident>::new();

    let bytes = s.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => {
                // Find the closing `}`.
                let start = i + 1;
                i += 1;
                while i < bytes.len() && bytes[i] != b'}' {
                    if bytes[i] == b'{' {
                        return Err(syn::Error::new(
                            path.span(),
                            "invalid http path: nested `{` in path params",
                        ));
                    }
                    i += 1;
                }
                if i >= bytes.len() {
                    return Err(syn::Error::new(
                        path.span(),
                        "invalid http path: unclosed `{` in path params",
                    ));
                }

                let end = i;
                let name = s[start..end].trim();
                if name.is_empty() {
                    return Err(syn::Error::new(
                        path.span(),
                        "invalid http path: empty `{}` path parameter",
                    ));
                }

                let ident: Ident = parse_str(name).map_err(|e| {
                    syn::Error::new(
                        path.span(),
                        format!(
                            "invalid http path parameter name `{name}`; must be a valid Rust identifier: {e}"
                        ),
                    )
                })?;

                if params.iter().any(|p| p == &ident) {
                    return Err(syn::Error::new(
                        path.span(),
                        format!("duplicate http path parameter `{ident}`"),
                    ));
                }

                params.push(ident);
                i += 1; // consume `}`
            }
            b'}' => {
                return Err(syn::Error::new(
                    path.span(),
                    "invalid http path: stray `}` in path params",
                ));
            }
            _ => i += 1,
        }
    }

    Ok(params)
}

pub struct GeneratedHttp {
    pub routes: Vec<GeneratedRoute>,
}

impl From<Http> for GeneratedHttp {
    fn from(http: Http) -> Self {
        Self {
            routes: http.routes.into_iter().map(GeneratedRoute::from).collect(),
        }
    }
}

pub struct GeneratedRoute {
    pub path: LitStr,
    #[allow(dead_code)]
    pub params: Option<Vec<Ident>>,
    pub method: Ident,
    pub handler_name: Ident,
    pub request: Path,
    pub reply: Path,
}

impl From<Route> for GeneratedRoute {
    fn from(route: Route) -> Self {
        // TODO: fix this hack to derive a handler method name from the request type name
        let ident = Ident::new("Request", Span::call_site());
        let segment = &PathSegment::from(ident);
        let request = route.request.segments.last().unwrap_or(segment);
        let request_str = quote! {#request}.to_string();
        let handler_name =
            request_str.strip_suffix("Request").unwrap_or(&request_str).to_lowercase();

        Self {
            path: route.path,
            params: Some(route.params),
            method: route.method,
            handler_name: format_ident!("{handler_name}"),
            request: route.request,
            reply: route.reply,
        }
    }
}

pub fn expand(http: &GeneratedHttp, client: &TokenStream) -> TokenStream {
    let routes = http.routes.iter().map(expand_route);
    let handlers = http.routes.iter().map(|r| expand_handler(r, client));

    quote! {
        mod http {
            use warp_sdk::api::{HttpResult, Reply};

            use super::*;

            pub struct Http;
            wasip3::http::proxy::export!(Http);

            impl wasip3::exports::http::handler::Guest for Http {
                #[wasi_otel::instrument]
                async fn handle(
                    request: wasip3::http::types::Request,
                ) -> core::result::Result<wasip3::http::types::Response, wasip3::http::types::ErrorCode> {
                    let router = axum::Router::new()
                        #(#routes)*;
                    wasi_http::serve(router, request).await
                }
            }

            #(#handlers)*
        }
    }
}

fn expand_route(route: &GeneratedRoute) -> TokenStream {
    let path = &route.path;
    let method = &route.method;
    let handler_name = &route.handler_name;

    quote! {
        .route(#path, axum::routing::#method(#handler_name))
    }
}

fn expand_handler(route: &GeneratedRoute, client: &TokenStream) -> TokenStream {
    let handler_name = &route.handler_name;
    let request = &route.request;
    let reply = &route.reply;

    let empty = &Vec::<Ident>::new();
    let params = route.params.as_ref().unwrap_or(empty);

    // generate handler function name and signature
    let handler_fn = if route.method == "get" {
        let param_args = if params.is_empty() {
            quote! {}
        } else if params.len() == 1 {
            quote! { axum::extract::Path(#(#params),*): axum::extract::Path<String> }
        } else {
            let mut param_types = Vec::new();
            for _ in 0..params.len() {
                param_types.push(format_ident!("String"));
            }

            quote! { axum::extract::Path((#(#params),*)): axum::extract::Path<(#(#param_types),*)> }
        };

        quote! { #handler_name(#param_args) }
    } else {
        quote! { #handler_name(body: bytes::Bytes) }
    };

    // generate request parameter and type
    let request = if route.method == "get" {
        if params.is_empty() {
            quote! { #request }
            //  quote! { #request::try_from(()).context("parsing request")? }
        } else if params.len() == 1 {
            quote! { #request::try_from(#(#params),*).context("parsing request")? }
        } else {
            quote! { #request::try_from((#(#params),*)).context("parsing request")? }
        }
    } else {
        quote! { #request::try_from(body.as_ref()).context("parsing request")? }
    };

    quote! {
        #[wasi_otel::instrument]
        async fn #handler_fn -> HttpResult<Reply<#reply>> {
            let client = #client;
            let request = #request;
            let reply = client.request(request).await.context("processing request")?;
            Ok(reply)
        }
    }
}
