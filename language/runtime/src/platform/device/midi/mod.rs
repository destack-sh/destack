#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

mod core;
mod host;
pub mod native;
mod selector;
pub(crate) mod simulation;
mod state;
#[cfg(test)]
mod tests;
pub mod vm;

pub(crate) use state::*;
