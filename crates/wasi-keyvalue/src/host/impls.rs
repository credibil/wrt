mod atomics;
mod batch;
mod store;

use crate::host::generated::wasi::keyvalue::store::Error;

pub type Result<T, E = Error> = anyhow::Result<T, E>;
