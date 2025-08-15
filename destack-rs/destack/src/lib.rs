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

pub use crate::basics::*;
pub use crate::core::*;
pub use crate::imagination::*;
pub use crate::presentation::*;
pub use crate::production::*;
pub use crate::simulation::*;

#[cfg(test)]
mod test;
