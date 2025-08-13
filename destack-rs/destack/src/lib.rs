#![feature(register_tool)]
#![register_tool(destack)]

mod core;
mod simulation;

pub use core::*;
pub use simulation::*;

#[cfg(test)]
mod test;
