use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;

use anyhow::Result;
use futures::future::BoxFuture;

// use crate::host::generated::wasi::vault::{ContainerMetadata, ObjectMetadata};

pub type FutureResult<T> = BoxFuture<'static, Result<T>>;

/// Blobstore providers implement the [`Client`] trait to allow the host to
/// connect to a backend (Azure Storage, NATS object store, etc) and open
/// containers.
pub trait Client: Debug + Send + Sync + 'static {
    /// The name of the backend this client is implemented for.
    fn name(&self) -> &'static str;

    /// Open a container.
    fn open(&self, identifier: String) -> FutureResult<Arc<dyn Locker>>;
}

/// [`ClientProxy`] provides a concrete wrapper around a `dyn Client` object.
/// It is used to store client resources in the resource table.
#[derive(Clone, Debug)]
pub struct ClientProxy(pub Arc<dyn Client>);

impl Deref for ClientProxy {
    type Target = Arc<dyn Client>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Providers implement the [`Locker`] trait to allow the host to
/// interact with different backend lockers (stores).
pub trait Locker: Debug + Send + Sync + 'static {
    /// The name of the locker.
    fn identifier(&self) -> String;

    /// Get the value associated with the key.
    fn get(&self, secret_id: String) -> FutureResult<Option<Vec<u8>>>;

    /// Set the value associated with the key.
    fn set(&self, secret_id: String, value: Vec<u8>) -> FutureResult<()>;

    /// Delete the value associated with the key.
    fn delete(&self, secret_id: String) -> FutureResult<()>;

    /// Check if the entry exists.
    fn exists(&self, secret_id: String) -> FutureResult<bool>;

    /// List all keys in the bucket.
    fn list_ids(&self) -> FutureResult<Vec<String>>;
}

/// Represents a locker resource in the WASI Vault.
#[derive(Debug)]
pub struct LockerProxy(pub Arc<dyn Locker>);

impl Deref for LockerProxy {
    type Target = Arc<dyn Locker>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
