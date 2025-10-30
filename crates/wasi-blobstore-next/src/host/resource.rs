use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;

use anyhow::Result;
use futures::future::BoxFuture;

use crate::host::generated::wasi::blobstore::container::{ContainerMetadata, ObjectMetadata};

pub type FutureResult<T> = BoxFuture<'static, Result<T>>;

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
