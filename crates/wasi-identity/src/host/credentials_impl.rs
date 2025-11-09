use anyhow::Context;
use wasmtime::component::{Accessor, Resource, ResourceTableError};

use crate::host::generated::wasi::identity::credentials::{
    AccessToken, Host, HostIdentity, HostIdentityWithStore, HostWithStore,
};
use crate::host::generated::wasi::identity::types::Error;
use crate::host::resource::IdentityProxy;
use crate::host::{WasiIdentity, WasiIdentityCtxView};

pub type Result<T, E = Error> = anyhow::Result<T, E>;

impl HostWithStore for WasiIdentity {
    async fn get_identity<T>(
        accessor: &Accessor<T, Self>, name: String,
    ) -> Result<Resource<IdentityProxy>> {
        let identity = accessor.with(|mut store| store.get().ctx.get_identity(name)).await?;
        let proxy = IdentityProxy(identity);
        Ok(accessor.with(|mut store| store.get().table.push(proxy))?)
    }
}

impl HostIdentityWithStore for WasiIdentity {
    async fn get_token<T>(
        accessor: &Accessor<T, Self>, self_: Resource<IdentityProxy>, scope: Vec<String>,
    ) -> Result<AccessToken> {
        let identity = get_identity(accessor, &self_)?;
        let token = identity.0.get_token(scope).await.context("issue getting access token")?;
        Ok(token)
    }

    async fn drop<T>(
        accessor: &Accessor<T, Self>, rep: Resource<IdentityProxy>,
    ) -> anyhow::Result<()> {
        accessor.with(|mut store| store.get().table.delete(rep).map(|_| Ok(())))?
    }
}

impl Host for WasiIdentityCtxView<'_> {}
impl HostIdentity for WasiIdentityCtxView<'_> {}

pub fn get_identity<T>(
    accessor: &Accessor<T, WasiIdentity>, self_: &Resource<IdentityProxy>,
) -> Result<IdentityProxy> {
    accessor.with(|mut store| {
        let identity = store.get().table.get(self_).map_err(|_e| Error::NoSuchIdentity)?;
        Ok::<_, Error>(identity.clone())
    })
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Self::InternalFailure(err.to_string())
    }
}

impl From<ResourceTableError> for Error {
    fn from(err: ResourceTableError) -> Self {
        Self::InternalFailure(err.to_string())
    }
}
