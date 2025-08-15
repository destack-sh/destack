#![feature(register_tool)]
#![feature(custom_inner_attributes)]
#![register_tool(destack)]

mod basics;
mod core;
mod distribution;
mod imagination;
mod presentation;
mod production;
mod simulation;

pub use basics::*;
pub use core::*;
pub use distribution::*;
pub use imagination::*;
pub use presentation::*;
pub use production::*;
pub use simulation::*;

#[cfg(test)]
mod test;
