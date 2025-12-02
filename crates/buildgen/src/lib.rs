use std::collections::HashMap;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Token, parse_macro_input};

/// Configuration for the runtime macro.
///
/// Parses input in the format:
/// ```ignore
/// {
///     wasi_http: WasiHttpCtx,
///     wasi_otel: DefaultOtelCtx,
///     ...
/// }
/// ```
struct RuntimeConfig {
    wasi_tuple: HashMap<String, syn::Type>,
}

impl Parse for RuntimeConfig {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        syn::braced!(content in input);

        let mut wasi_tuple = HashMap::new();

        while !content.is_empty() {
            // Parse identifier (e.g., wasi_http, wasi_otel, etc.)
            let wasi_ident: Ident = content.parse()?;
            let wasi_str = wasi_ident.to_string();

            // Extract the component name by removing "wasi_" prefix if present
            let component = if let Some(name) = wasi_str.strip_prefix("wasi_") {
                name.to_string()
            } else {
                wasi_str
            };

            // Parse ":"
            content.parse::<Token![:]>()?;

            // Parse BackendType
            let backend: syn::Type = content.parse()?;

            wasi_tuple.insert(component, backend);

            // Parse optional comma
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }

        Ok(Self { wasi_tuple })
    }
}

/// Information about a WASI interface and its configuration.
struct ComponentInfo {
    name: String,
    backend_type: syn::Type,
    view_trait: syn::Ident,
    view_method: syn::Ident,
    ctx_view_type: syn::Ident,
    is_server: bool,
}

impl ComponentInfo {
    fn new(name: &str, backend_type: syn::Type) -> Self {
        let view_trait = syn::parse_str(&format!("Wasi{}View", capitalize(name))).unwrap();
        let view_method = syn::parse_str(name).unwrap();
        let ctx_view_type = syn::parse_str(&format!("Wasi{}CtxView", capitalize(name))).unwrap();
        let is_server = matches!(name, "http" | "messaging" | "websockets");

        Self {
            name: name.to_string(),
            backend_type,
            view_trait,
            view_method,
            ctx_view_type,
            is_server,
        }
    }

    /// Returns true if this interface requires a backend connection
    fn needs_backend(&self) -> bool {
        // !matches!(self.name.as_str(), "http" | "websockets")
        true
    }
}

fn capitalize(s: &str) -> String {
    // Handle special cases for multi-word interface names
    match s {
        "keyvalue" => "KeyValue".to_string(),
        "blobstore" => "Blobstore".to_string(),
        "websockets" => "WebSockets".to_string(),
        _ => {
            let mut chars = s.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        }
    }
}

/// Generates the runtime infrastructure based on the configuration.
///
/// # Example
///
/// ```ignore
/// buildgen::runtime!({
///     wasi_http: WasiHttpCtx,
///     wasi_otel: DefaultOtelCtx,
///     wasi_blobstore: MongoDb,
///     wasi_keyvalue: Nats,
///     wasi_messaging: Nats,
///     wasi_vault: Azure
/// });
/// ```
///
/// This will generate:
/// - A `RuntimeContext` struct with fields for each unique backend
/// - A `RuntimeStoreCtx` struct with fields for each interface
/// - Implementation of the `State` trait
/// - WASI view trait implementations
/// - A `run` function that starts the runtime
#[proc_macro]
pub fn runtime(input: TokenStream) -> TokenStream {
    let config = parse_macro_input!(input as RuntimeConfig);

    // Collect unique backends (only for interfaces that need them)
    let mut unique_backends = HashMap::<String, syn::Type>::new();
    let mut components = Vec::<ComponentInfo>::new();

    for (component_name, backend_type) in &config.wasi_tuple {
        let component = ComponentInfo::new(component_name, backend_type.clone());

        // Track unique backend types only for interfaces that need backend connections
        if component.needs_backend() {
            let backend_key = quote!(#backend_type).to_string();
            unique_backends.insert(backend_key.clone(), backend_type.clone());
        }

        components.push(component);
    }

    // Generate Context struct fields
    let context_fields: Vec<_> = unique_backends
        .values()
        .map(|backend_type| {
            let field_name = backend_field_name(backend_type);
            quote! {
                #field_name: #backend_type
            }
        })
        .collect();

    // Generate Context initialization in new()
    let context_inits: Vec<_> = unique_backends
        .values()
        .map(|backend_type| {
            let field_name = backend_field_name(backend_type);
            quote! {
                #field_name: #backend_type::connect().await?
            }
        })
        .collect();

    // Generate StoreCtx fields
    let store_ctx_fields: Vec<_> = components
        .iter()
        .map(|component| {
            let field_name = syn::parse_str::<syn::Ident>(&component.name).unwrap();
            let ctx_type = &component.backend_type;
            quote! {
                pub #field_name: #ctx_type
            }
        })
        .collect();

    // Generate StoreCtx initialization in new_store()
    let store_ctx_inits: Vec<_> = components
        .iter()
        .map(|component| {
            let field_name = syn::parse_str::<syn::Ident>(&component.name).unwrap();

            // Special handling for different context types
            match component.name.as_str() {
                "http" => quote! {
                    #field_name: wasi_http::WasiHttpCtx
                },
                "websockets" => quote! {
                    #field_name: wasi_websockets::DefaultWebSocketsCtx
                },
                _ => {
                    let backend_field = backend_field_name(&component.backend_type);
                    quote! {
                        #field_name: self.#backend_field.clone()
                    }
                }
            }
        })
        .collect();

    // Generate WASI interface linking
    let link_interfaces: Vec<_> = components
        .iter()
        .map(|component| {
            let name = &component.name;
            match name.as_str() {
                "http" => quote! {
                    compiled.link(wasi_http::WasiHttp)?;
                },
                "otel" => quote! {
                    compiled.link(wasi_otel::WasiOtel)?;
                },
                "blobstore" => quote! {
                    compiled.link(wasi_blobstore::WasiBlobstore)?;
                },
                "keyvalue" => quote! {
                    compiled.link(wasi_keyvalue::WasiKeyValue)?;
                },
                "messaging" => quote! {
                    compiled.link(wasi_messaging::WasiMessaging)?;
                },
                "vault" => quote! {
                    compiled.link(wasi_vault::WasiVault)?;
                },
                "sql" => quote! {
                    compiled.link(wasi_sql::WasiSql)?;
                },
                "identity" => quote! {
                    compiled.link(wasi_identity::WasiIdentity)?;
                },
                "websockets" => quote! {
                    compiled.link(wasi_websockets::WasiWebSockets)?;
                },
                _ => quote! {},
            }
        })
        .collect();

    // Generate server start calls
    let server_starts: Vec<_> = components
        .iter()
        .filter(|component| component.is_server)
        .map(|component| {
            let name = &component.name;
            match name.as_str() {
                "http" => quote! {
                    Box::pin(wasi_http::WasiHttp.run(self))
                },
                "messaging" => quote! {
                    Box::pin(wasi_messaging::WasiMessaging.run(self))
                },
                "websockets" => quote! {
                    Box::pin(wasi_websockets::WasiWebSockets.run(self))
                },
                _ => quote! {},
            }
        })
        .collect();

    // Generate WASI view implementations
    let view_impls: Vec<_> = components
        .iter()
        .map(|component| {
            let view_trait = &component.view_trait;
            let view_method = &component.view_method;
            let ctx_view_type = &component.ctx_view_type;
            let field_name = syn::parse_str::<syn::Ident>(&component.name).unwrap();

            let module_prefix = match component.name.as_str() {
                "http" => quote!(wasi_http),
                "otel" => quote!(wasi_otel),
                "blobstore" => quote!(wasi_blobstore),
                "keyvalue" => quote!(wasi_keyvalue),
                "messaging" => quote!(wasi_messaging),
                "vault" => quote!(wasi_vault),
                "sql" => quote!(wasi_sql),
                "identity" => quote!(wasi_identity),
                "websockets" => {
                    // Special case for websockets which has a different trait name
                    return quote! {
                        impl wasi_websockets::WebSocketsView for RuntimeStoreCtx {
                            fn start(&mut self) -> wasi_websockets::WasiWebSocketsCtxView<'_> {
                                wasi_websockets::WasiWebSocketsCtxView {
                                    ctx: &mut self.websockets,
                                    table: &mut self.table,
                                }
                            }
                        }
                    };
                }
                _ => return quote! {},
            };

            quote! {
                impl #module_prefix::#view_trait for RuntimeStoreCtx {
                    fn #view_method(&mut self) -> #module_prefix::#ctx_view_type<'_> {
                        #module_prefix::#ctx_view_type {
                            ctx: &mut self.#field_name,
                            table: &mut self.table,
                        }
                    }
                }
            }
        })
        .collect();

    let expanded = quote! {
        use runtime::{Resource, Server};

        /// Runtime state holding pre-instantiated components and backend connections.
        #[derive(Clone)]
        struct RuntimeContext {
            instance_pre: wasmtime::component::InstancePre<RuntimeStoreCtx>,
            #(#context_fields,)*
        }

        impl RuntimeContext {
            /// Creates a new runtime state by linking WASI interfaces and connecting to backends.
            async fn new(compiled: &mut runtime::Compiled<RuntimeStoreCtx>) -> anyhow::Result<Self> {
                // Link all enabled WASI interfaces to the component
                #(#link_interfaces)*

                Ok(Self {
                    instance_pre: compiled.pre_instantiate()?,
                    #(#context_inits,)*
                })
            }

            /// Starts all enabled server interfaces (HTTP, messaging, websockets).
            async fn start(&self) -> anyhow::Result<()> {
                use futures::future::{BoxFuture, try_join_all};

                let futures: Vec<BoxFuture<'_, anyhow::Result<()>>> = vec![
                    #(#server_starts,)*
                ];
                try_join_all(futures).await?;
                Ok(())
            }
        }

        impl runtime::State for RuntimeContext {
            type StoreCtx = RuntimeStoreCtx;

            fn instance_pre(&self) -> &wasmtime::component::InstancePre<Self::StoreCtx> {
                &self.instance_pre
            }

            fn new_store(&self) -> Self::StoreCtx {
                use wasmtime_wasi::{WasiCtxBuilder, ResourceTable};

                let wasi_ctx = WasiCtxBuilder::new()
                    .inherit_args()
                    .inherit_env()
                    .inherit_stdin()
                    .stdout(tokio::io::stdout())
                    .stderr(tokio::io::stderr())
                    .build();

                RuntimeStoreCtx {
                    table: ResourceTable::new(),
                    wasi: wasi_ctx,
                    #(#store_ctx_inits,)*
                }
            }
        }

        /// Per-instance data shared between the WebAssembly runtime and host functions.
        pub struct RuntimeStoreCtx {
            pub table: wasmtime_wasi::ResourceTable,
            pub wasi: wasmtime_wasi::WasiCtx,
            #(#store_ctx_fields,)*
        }

        // WASI View Implementations
        impl wasmtime_wasi::WasiView for RuntimeStoreCtx {
            fn ctx(&mut self) -> wasmtime_wasi::WasiCtxView<'_> {
                wasmtime_wasi::WasiCtxView {
                    ctx: &mut self.wasi,
                    table: &mut self.table,
                }
            }
        }

        #(#view_impls)*

        /// Run the specified wasm guest using the configured runtime.
        pub async fn runtime_run(wasm: std::path::PathBuf) -> anyhow::Result<()> {
            use anyhow::Context as _;

            runtime_cli::RuntimeConfig::from_wasm(&wasm)?;

            let mut compiled = runtime::Runtime::new()
                .build(&wasm)
                .with_context(|| format!("compiling {}", wasm.display()))?;
            let run_state = RuntimeContext::new(&mut compiled)
                .await
                .context("preparing runtime state")?;
            run_state.start().await.context("starting runtime services")
        }
    };

    TokenStream::from(expanded)
}

/// Generates a field name for a backend type.
fn backend_field_name(backend_type: &syn::Type) -> syn::Ident {
    // Extract the last identifier from the type path
    let name = if let syn::Type::Path(type_path) = backend_type {
        if let Some(segment) = type_path.path.segments.last() {
            segment.ident.to_string().to_lowercase()
        } else {
            "backend".to_string()
        }
    } else {
        "backend".to_string()
    };

    syn::parse_str(&name).unwrap()
}
