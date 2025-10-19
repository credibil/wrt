use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;

use anyhow::Result;
use futures::future::BoxFuture;

use crate::host::generated::wasi::blobstore::container::{ContainerMetadata, ObjectMetadata};

pub type FutureResult<T> = BoxFuture<'static, Result<T>>;

/// Blobstore providers implement the [`Client`] trait to allow the host to
/// connect to a backend (Azure Storage, NATS object store, etc) and open
/// containers.
pub trait Client: Debug + Send + Sync + 'static {
    /// The name of the backend this client is implemented for.
    fn name(&self) -> &'static str;

    /// Open a container.
    fn create_container(&self, name: String) -> FutureResult<Arc<dyn Container>>;

    /// Get a container.
    fn get_container(&self, name: String) -> FutureResult<Arc<dyn Container>>;

    /// Delete a container.
    fn delete_container(&self, name: String) -> FutureResult<()>;

    /// Check if a container exists.
    fn container_exists(&self, name: String) -> FutureResult<bool>;
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

/// Providers implement the [`Container`] trait to allow the host to
/// interact with different backend containers.
pub trait Container: Debug + Send + Sync + 'static {
    /// The name of the container.
    fn name(&self) -> FutureResult<String>;

    fn info(&self) -> FutureResult<ContainerMetadata>;

    /// Get the value associated with the key.
    fn get_data(&self, name: String, _start: u64, _end: u64) -> FutureResult<Option<Vec<u8>>>;

    /// Set the value associated with the key.
    fn write_data(&self, name: String, data: Vec<u8>) -> FutureResult<()>;

    /// List all objects in the container.
    fn list_objects(&self) -> FutureResult<Vec<String>>;

    /// Delete the value associated with the key.
    fn delete_object(&self, name: String) -> FutureResult<()>;

    /// Check if the object exists.
    fn has_object(&self, name: String) -> FutureResult<bool>;

    fn object_info(&self, name: String) -> FutureResult<ObjectMetadata>;
}

#[derive(Clone, Debug)]
pub struct ContainerProxy(pub Arc<dyn Container>);

impl Deref for ContainerProxy {
    type Target = Arc<dyn Container>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
