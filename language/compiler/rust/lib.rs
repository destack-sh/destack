#![feature(default_field_values)]
#![feature(if_let_guard)]

mod compile;
mod cranelift;
mod diagnostic;
mod dir;
mod execute;
mod mir;
mod resolve;
mod validate;
mod wasm;

pub use compile::*;
