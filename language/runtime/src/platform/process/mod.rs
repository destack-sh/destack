#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

#[path = "abi.generated.rs"]
pub(crate) mod abi_generated;
pub(crate) use crate::platform::thread::{
    ThreadCpuSet as ProcessCpuSet, ThreadCpuSetVm as ProcessCpuSetVm,
};
pub(crate) use abi_generated::*;
pub mod core;
mod host;
