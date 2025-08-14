#![feature(register_tool)]
#![feature(custom_inner_attributes)]
#![register_tool(destack)]

mod core;
mod simulation;

pub use core::*;
pub use simulation::*;

#[cfg(test)]
mod test;
