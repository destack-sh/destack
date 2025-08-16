//! destack.simulation.physics@2025.08.15.1

#![destack::partial(destack.simulation.physics, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::simulation::physics::_gen::*;
pub use crate::simulation::physics::body::*;
pub use crate::simulation::physics::collider::*;
pub use crate::simulation::physics::joint::*;
pub use crate::simulation::physics::rigid::*;
pub use crate::simulation::physics::soft::*;

pub mod _gen;
pub mod body;
pub mod collider;
pub mod joint;
pub mod rigid;
pub mod soft;
