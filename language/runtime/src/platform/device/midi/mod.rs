#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

mod core;
pub(crate) mod host;
pub mod native;
mod selector;
pub(crate) mod simulation;
#[cfg(test)]
mod tests;
pub mod vm;
