use anyhow::anyhow;
use wasmtime::component::Resource;

use crate::host::generated::wasi::keyvalue::atomics;
use crate::host::generated::wasi::keyvalue::atomics::CasError;
use crate::host::generated::wasi::keyvalue::store::Error;
use crate::host::resource::{BucketProxy, Cas};
use crate::host::{Host, Result};

impl atomics::HostCas for Host<'_> {
    /// Construct a new CAS operation. Implementors can map the underlying functionality
    /// (transactions, versions, etc) as desired.
    async fn new(&mut self, bucket: Resource<BucketProxy>, key: String) -> Result<Resource<Cas>> {
        let Ok(bucket) = self.table.get(&bucket) else {
            return Err(Error::NoSuchStore);
        };
        let current =
            bucket.get(key.clone()).await.map_err(|e| anyhow!("issue getting key: {e}"))?;
        let cas = Cas { key, current };

        Ok(self.table.push(cas)?)
    }

    /// Get the current value of the CAS handle.
    async fn current(&mut self, self_: Resource<Cas>) -> Result<Option<Vec<u8>>> {
        let Ok(cas) = self.table.get(&self_) else {
            return Err(Error::NoSuchStore);
        };
        let value = cas.current.clone();
        Ok(value)
    }

    /// Drop the CAS handle.
    async fn drop(&mut self, rep: Resource<Cas>) -> anyhow::Result<()> {
        tracing::trace!("atomics::HostCas::drop");
        self.table.delete(rep).map(|_| Ok(()))?
    }
}

impl atomics::Host for Host<'_> {
    /// Atomically increment the value associated with the key in the store by
    /// the given delta. It returns the new value.
    ///
    /// If the key does not exist in the store, it creates a new key-value pair
    /// with the value set to the given delta.
    ///
    /// If any other error occurs, it returns an `Err(error)`.
    async fn increment(
        &mut self, bucket: Resource<BucketProxy>, key: String, delta: i64,
    ) -> Result<i64> {
        let Ok(bucket) = self.table.get_mut(&bucket) else {
            return Err(Error::NoSuchStore);
        };

        let Ok(Some(value)) = bucket.get(key.clone()).await else {
            return Err(anyhow!("no value for {key}").into());
        };

        // increment value by delta
        let slice: &[u8] = &value;
        let mut buf = [0u8; 8];
        let len = 8.min(slice.len());
        buf[..len].copy_from_slice(&slice[..len]);
        let inc = i64::from_be_bytes(buf) + delta;

        // update value in bucket
        if let Err(e) = bucket.set(key, inc.to_be_bytes().to_vec()).await {
            return Err(anyhow!("issue saving increment: {e}").into());
        }

        Ok(inc)
    }

    /// Perform the swap on a CAS operation. This consumes the CAS handle and
    /// returns an error if the CAS operation failed.
    async fn swap(
        &mut self, _self_: Resource<Cas>, _value: Vec<u8>,
    ) -> anyhow::Result<Result<(), CasError>> {
        Err(anyhow!("not implemented"))
    }
}
