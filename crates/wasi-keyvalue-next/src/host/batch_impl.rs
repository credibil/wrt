use crate::WasiKeyValueCtxView;
use crate::host::Result;
use crate::host::WasiKeyValue;
use crate::host::generated::wasi::keyvalue::batch::{Host, HostWithStore};
use crate::host::resource::BucketProxy;
use crate::host::store_impl::get_bucket;
use anyhow::anyhow;
use wasmtime::component::Accessor;
use wasmtime::component::Resource;

impl HostWithStore for WasiKeyValue {
    async fn get_many<T>(
        accessor: &Accessor<T, Self>, self_: Resource<BucketProxy>, keys: Vec<String>,
    ) -> Result<Vec<Option<(String, Vec<u8>)>>> {
        let bucket = get_bucket(accessor, &self_)?;

        let mut many = Vec::new();
        for key in keys {
            if let Some(value) = bucket.get(key.clone()).await? {
                many.push(Some((key, value)));
            }
        }

        Ok(many)
    }

    async fn set_many<T>(
        accessor: &Accessor<T, Self>, self_: Resource<BucketProxy>,
        key_values: Vec<(String, Vec<u8>)>,
    ) -> Result<()> {
        let bucket = get_bucket(accessor, &self_)?;
        for (key, value) in key_values {
            bucket.set(key, value).await?;
        }
        Ok(())
    }

    async fn delete_many<T>(
        accessor: &Accessor<T, Self>, self_: Resource<BucketProxy>, keys: Vec<String>,
    ) -> Result<()> {
        let bucket = get_bucket(accessor, &self_)?;
        for key in keys {
            if let Err(e) = bucket.delete(key).await {
                return Err(anyhow!("issue deleting value: {e}").into());
            }
        }
        Ok(())
    }
}

impl Host for WasiKeyValueCtxView<'_> {}
