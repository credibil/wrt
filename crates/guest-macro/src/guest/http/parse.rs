use quote::format_ident;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitStr, Result, Token, Type, parse_str};

pub struct Http {
    pub routes: Vec<Route>,
}

impl Parse for Http {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut routes = Vec::new();
        while !input.is_empty() {
            let route: Route = input.parse()?;
            routes.push(route);

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(Self { routes })
    }
}
pub struct Route {
    pub path: LitStr,
    pub params: Vec<Ident>,
    pub method: Ident,
    pub request: Type,
    pub reply: Type,
}

impl Parse for Route {
    fn parse(input: ParseStream) -> Result<Self> {
        let path: LitStr = input.parse()?;
        input.parse::<Token![:]>()?;

        let settings;
        syn::braced!(settings in input);

        let mut method: Option<Ident> = None;
        let mut request: Option<syn::Type> = None;
        let mut reply: Option<syn::Type> = None;

        while !settings.is_empty() {
            let key: Ident = settings.parse()?;
            settings.parse::<Token![:]>()?;

            if key == "method" {
                method = Some(settings.parse()?);
            } else if key == "request" {
                request = Some(settings.parse()?);
            } else if key == "reply" {
                reply = Some(settings.parse()?);
            } else {
                return Err(syn::Error::new(
                    key.span(),
                    "unknown http field; expected `method`, `request`, or `reply`",
                ));
            }

            if settings.peek(Token![,]) {
                settings.parse::<Token![,]>()?;
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
            return Err(syn::Error::new(path.span(), "http route is missing `request`"));
        };
        let Some(reply) = reply else {
            return Err(syn::Error::new(path.span(), "http route is missing `reply`"));
        };
        let params = parse_path_params(&path)?;

        Ok(Self {
            path,
            params,
            method,
            request,
            reply,
        })
    }
}

fn parse_path_params(path: &LitStr) -> Result<Vec<Ident>> {
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
