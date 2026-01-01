use quote::{format_ident, quote};
use syn::{Ident, LitStr, Type};

use crate::guest as parsed;

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

pub fn generate(http: parsed::Http) -> HttpGuest {
    HttpGuest {
        routes: http.routes.into_iter().map(generate_route).collect(),
    }
}

fn generate_route(route: parsed::Route) -> Route {
    let request = route.request;
    let request_str = quote! {#request}.to_string();
    let handler_name = request_str
        .strip_suffix("Request")
        .unwrap_or(&request_str)
        .to_lowercase();

    Route {
        path: route.path,
        params: Some(route.params),
        method: route.method,
        handler_name: format_ident!("{handler_name}"),
        request,
        reply: route.reply,
    }
}
