//! destack.simulation.physics@2025.08.15.1

#![destack::partial(simulation/physics, file)]
#![allow(unused_imports)]

pub use crate::simulation::physics::collider::*;
pub use crate::simulation::physics::_gen::*;
pub use crate::simulation::physics::soft::*;
pub use crate::simulation::physics::body::*;
pub use crate::simulation::physics::rigid::*;
pub use crate::simulation::physics::joint::*;

mod collider;
mod _gen;
mod soft;
mod body;
mod rigid;
mod joint;

pub(crate) use crate::simulation::physics::_gen::*;

pub(crate) use crate::simulation::physics::body::*;

pub(crate) use crate::simulation::physics::collider::*;

pub(crate) use crate::simulation::physics::soft::*;

pub(crate) use crate::simulation::physics::_gen::*;

pub(crate) use crate::simulation::physics::body::*;

pub(crate) use crate::simulation::physics::collider::*;

pub(crate) use crate::simulation::physics::soft::*;

pub(crate) use crate::simulation::physics::_gen::*;

pub(crate) use crate::simulation::physics::body::*;

pub(crate) use crate::simulation::physics::collider::*;

pub(crate) use crate::simulation::physics::soft::*;