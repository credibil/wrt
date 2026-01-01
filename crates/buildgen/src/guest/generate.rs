use syn::{LitStr, Type};

use crate::guest::Input;
use crate::guest::http::generate::HttpGuest;
use crate::guest::messaging::generate::MessagingGuest;

pub struct Generated {
    pub owner: LitStr,
    pub provider: Type,
    pub http: Option<HttpGuest>,
    pub messaging: Option<MessagingGuest>,
}

impl TryFrom<Input> for Generated {
    type Error = syn::Error;

    fn try_from(input: Input) -> Result<Self, Self::Error> {
        let http = input.http.map(HttpGuest::from);
        let messaging = input.messaging.map(MessagingGuest::from);

        Ok(Self {
            owner: input.owner,
            provider: input.provider,
            http,
            messaging,
        })
    }
}
