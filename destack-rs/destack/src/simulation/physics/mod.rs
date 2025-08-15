//! destack.simulation.physics@2025.08.15.1

#![destack::partial(destack.simulation.physics, file)]
#![allow(unused_imports)]

pub use crate::simulation::physics::_gen::*;
pub use crate::simulation::physics::body::*;
pub use crate::simulation::physics::collider::*;
pub use crate::simulation::physics::joint::*;
pub use crate::simulation::physics::rigid::*;
pub use crate::simulation::physics::soft::*;

mod _gen;
mod body;
mod collider;
mod joint;
mod rigid;
mod soft;
