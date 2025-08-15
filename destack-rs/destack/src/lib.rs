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

pub use destack_fractional::*;
pub use destack_json::*;
pub use destack_time::*;
pub use destack_uuid::*;

pub use std::collections::HashMap;

#[cfg(test)]
mod test;
