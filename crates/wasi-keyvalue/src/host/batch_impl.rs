use anyhow::anyhow;
use wasmtime::component::Resource;

use crate::host::generated::wasi::keyvalue::batch;
use crate::host::generated::wasi::keyvalue::store::Error;
use crate::host::resource::BucketProxy;
use crate::host::{Host, Result};

impl batch::Host for Host<'_> {
    async fn get_many(
        &mut self, bucket: Resource<BucketProxy>, keys: Vec<String>,
    ) -> Result<Vec<Option<(String, Vec<u8>)>>> {
        let Ok(bucket) = self.table.get(&bucket) else {
            return Err(Error::NoSuchStore);
        };

        let mut many = Vec::new();
        for key in keys {
            let value =
                bucket.get(key.clone()).await.map_err(|e| anyhow!("issue getting value: {e}"))?;
            if let Some(value) = value {
                many.push(Some((key, value)));
            }
        }

        Ok(many)
    }

    async fn set_many(
        &mut self, bucket: Resource<BucketProxy>, key_values: Vec<(String, Vec<u8>)>,
    ) -> Result<()> {
        let Ok(bucket) = self.table.get_mut(&bucket) else {
            return Err(Error::NoSuchStore);
        };

        for (key, value) in key_values {
            if let Err(e) = bucket.set(key, value).await {
                return Err(anyhow!("issue saving value: {e}").into());
            }
        }

        Ok(())
    }

    async fn delete_many(
        &mut self, bucket: Resource<BucketProxy>, keys: Vec<String>,
    ) -> Result<()> {
        let Ok(bucket) = self.table.get_mut(&bucket) else {
            return Err(Error::NoSuchStore);
        };

        for key in keys {
            if let Err(e) = bucket.delete(key).await {
                return Err(anyhow!("issue deleting value: {e}").into());
            }
        }

        Ok(())
    }
}
