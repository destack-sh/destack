#![feature(register_tool)]
#![register_tool(destack)]

mod simulation;

pub use simulation::*;

#[cfg(test)]
mod test;
