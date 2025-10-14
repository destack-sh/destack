#![feature(default_field_values)]
#![feature(if_let_guard)]

mod check;
mod compile;
mod cranelift;
mod dir;
mod evaluate;
mod mir;

pub use compile::*;
