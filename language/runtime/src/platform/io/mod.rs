#[path = "abi.generated.rs"]
pub(crate) mod abi_generated;

pub(crate) use abi_generated::*;
mod state;
pub(crate) use state::*;
