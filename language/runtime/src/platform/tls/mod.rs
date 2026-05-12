#[path = "abi.generated.rs"]
pub(crate) mod abi_generated;

pub(crate) use abi_generated::*;
#[cfg(any(unix, windows))]
mod core;
mod host;
mod unsupported;
