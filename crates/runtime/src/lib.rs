#![cfg(not(target_arch = "wasm32"))]

//! # WebAssembly Runtime

mod cli;
#[cfg(feature = "jit")]
mod compiler;
mod runtime;
mod state;
mod traits;

pub use self::cli::*;
#[cfg(feature = "jit")]
pub use self::compiler::*;
pub use self::runtime::*;
pub use self::state::*;
pub use self::traits::*;
