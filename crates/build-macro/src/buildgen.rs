use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::{ Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::{Token, braced, token};

#[derive(Debug, Default)]
pub struct Config {
    components: Vec<Component>,
}

#[derive(Debug, Default)]
pub struct Component {
    pub name: String,
    pub resources: Vec<String>,
}

pub fn expand(input: &Config) -> Result<TokenStream> {
    // let mut src = match input.components.generate(&input.resolve, input.world) {
    //     Ok(s) => s,
    //     Err(e) => return Err(Error::new(Span::call_site(), e.to_string())),
    // };

    // println!("{:#?}", input);

    Ok(quote! {
        let Command::Run { wasm } = Cli::parse().command else {
            return Err(anyhow!("only run command is supported"));
        };
        let builder = RuntimeBuilder::new(wasm, true);
        tracing::info!("Tracing initialised, logging available");

        let (mongodb, nats, az_vault) =
            tokio::try_join!(MongoDb::new(), Nats::new(), AzKeyVault::new())?;

        let messaging = WasiMessaging.resource(nats.clone()).await?;
        let keyvalue = WasiKeyValue.resource(nats).await?;
        let blobstore = WasiBlobstore.resource(mongodb).await?;
        let vault = WasiVault.resource(az_vault).await?;

        let runtime = builder
            .register(WasiOtel)
            .register(WasiHttp)
            .register(blobstore)
            .register(keyvalue)
            .register(messaging)
            .register(vault)
            .build();

        return runtime.await;
    })
}

impl Parse for Config {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let call_site = Span::call_site();
        let mut config = Self::default();

        // content should be wrapped in braces
        if !input.peek(token::Brace) {
            return Err(input.error("expected JSON object"));
        }

        let content;
        braced!(content in input);

        // parse components
        let fields = Punctuated::<Component, token::Comma>::parse_terminated(&content)?;
        for field in fields.into_pairs() {
            let field = field.into_value();
            config.components.push(field);
        }

        Ok(config)
    }
}

impl Parse for Component {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut component = Self::default();

        // parse component name
        let name: syn::Ident = input.parse()?;
        component.name = name.to_string();

        input.parse::<Token![:]>()?;

        // parse resources
        let contents;
        syn::bracketed!(contents in input);
        let resources = Punctuated::<syn::Ident, Token![,]>::parse_terminated(&contents)?;

        for res in resources {
            component.resources.push(res.to_string());
        }

        Ok(component)
    }
}

mod kw {
    syn::custom_keyword!(inline);
    syn::custom_keyword!(path);
}

// enum Opt {
//     World(syn::LitStr),
//     Path(Vec<syn::LitStr>),
//     TrappableErrorType(Vec<TrappableError>),
//     Ownership(Ownership),
//     Interfaces(syn::LitStr),
//     With(HashMap<String, String>),
//     AdditionalDerives(Vec<syn::Path>),
//     Stringify(bool),
//     WasmtimeCrate(syn::Path),
//     IncludeGeneratedCodeFromFile(bool),
//     Imports(FunctionConfig, Span),
// }
