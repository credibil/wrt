use quote::{format_ident, quote};
use syn::{Ident, LitStr, Type};

use crate::guest::http::parse;

pub struct Http {
    pub routes: Vec<Route>,
}

impl From<parse::Http> for Http {
    fn from(http: parse::Http) -> Self {
        Self {
            routes: http.routes.into_iter().map(Route::from).collect(),
        }
    }
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

impl From<parse::Route> for Route {
    fn from(route: parse::Route) -> Self {
        let request = route.request;
        let request_str = quote! {#request}.to_string();
        let handler_name =
            request_str.strip_suffix("Request").unwrap_or(&request_str).to_lowercase();

        Self {
            path: route.path,
            params: Some(route.params),
            method: route.method,
            handler_name: format_ident!("{handler_name}"),
            request,
            reply: route.reply,
        }
    }
}
