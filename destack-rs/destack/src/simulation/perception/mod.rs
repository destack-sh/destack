//! destack.simulation.perception@2025.08.15.1

#![destack::partial(simulation/perception, file)]
#![allow(unused_imports)]

pub(crate) use crate::simulation::perception::_gen::*;
pub(crate) use crate::simulation::perception::clipboard::*;
pub(crate) use crate::simulation::perception::drag::*;
pub(crate) use crate::simulation::perception::focus::*;
pub(crate) use crate::simulation::perception::input::*;
pub(crate) use crate::simulation::perception::key::*;
pub use crate::simulation::perception::mouse::*;
pub(crate) use crate::simulation::perception::pointer::*;

mod _gen;
mod clipboard;
mod drag;
mod focus;
mod input;
mod key;
mod mouse;
mod pointer;
