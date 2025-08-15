//! simulation@2025.08.15.1

#![destack::partial(simulation, file)]

pub use perception::*;
pub use geometry::*;
pub use physics::*;

mod perception;
mod geometry;
mod physics;