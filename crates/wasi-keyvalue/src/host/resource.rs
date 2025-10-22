use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;

use anyhow::Result;
use futures::future::BoxFuture;

pub type FutureResult<T> = BoxFuture<'static, Result<T>>;

/// Key-Value providers implement the [`Client`] trait to allow the host to
/// connect to a backend (in-memory, redis, nats-kv, etc) and open buckets.
pub trait Client: Debug + Send + Sync + 'static {
    /// The name of the backend this client is implemented for.
    fn name(&self) -> &'static str;

    /// Open a bucket with the given identifier.
    fn open(&self, identifier: String) -> FutureResult<impl Bucket>;
}

// /// [`ClientProxy`] provides a concrete wrapper around a `dyn Client` object.
// /// It is used to store client resources in the resource table.
// #[derive(Clone, Debug)]
// pub struct ClientProxy<T: Client>(pub T);

// impl<T: Client> Deref for ClientProxy<T> {
//     type Target = T;

//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

/// Providers implement the [`Bucket`] trait to allow the host to
/// interact with different backend buckets (stores).
pub trait Bucket: Debug + Send + Sync + 'static {
    /// The name of the bucket.
    fn name(&self) -> &'static str;

    /// Get the value associated with the key.
    fn get(&self, key: String) -> FutureResult<Option<Vec<u8>>>;

    /// Set the value associated with the key.
    fn set(&self, key: String, value: Vec<u8>) -> FutureResult<()>;

    /// Delete the value associated with the key.
    fn delete(&self, key: String) -> FutureResult<()>;

    /// Check if the entry exists.
    fn exists(&self, key: String) -> FutureResult<bool>;

    /// List all keys in the bucket.
    fn keys(&self) -> FutureResult<Vec<String>>;
}

#[derive(Clone, Debug)]
pub struct BucketProxy(pub Arc<dyn Bucket>);

impl Deref for BucketProxy {
    type Target = Arc<dyn Bucket>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// CAS (Compare-And-Swap) operation handle.
#[derive(Clone, Debug)]
pub struct Cas {
    /// The key associated with the CAS operation.
    pub key: String,

    /// The current value associated with the key.
    pub current: Option<Vec<u8>>,
}
