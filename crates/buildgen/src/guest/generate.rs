use syn::{Ident, LitStr};

use crate::guest::Input;
use crate::guest::http::generate::Http;
use crate::guest::messaging::generate::Messaging;

pub struct Generated {
    pub owner: LitStr,
    pub provider: Ident,
    pub http: Option<Http>,
    pub messaging: Option<Messaging>,
}

impl TryFrom<Input> for Generated {
    type Error = syn::Error;

    fn try_from(input: Input) -> Result<Self, Self::Error> {
        Ok(Self {
            owner: input.owner,
            provider: input.provider,
            http: input.http.map(Http::from),
            messaging: input.messaging.map(Messaging::from),
        })
    }
}
