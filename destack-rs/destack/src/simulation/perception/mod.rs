//! destack.simulation.perception@2025.08.15.1

#![destack::partial(destack.simulation.perception, file)]
#![allow(unused_imports)]

pub use crate::simulation::perception::_gen::*;
pub use crate::simulation::perception::clipboard::*;
pub use crate::simulation::perception::drag::*;
pub use crate::simulation::perception::focus::*;
pub use crate::simulation::perception::input::*;
pub use crate::simulation::perception::key::*;
pub use crate::simulation::perception::mouse::*;
pub use crate::simulation::perception::pointer::*;

mod _gen;
mod clipboard;
mod drag;
mod focus;
mod input;
mod key;
mod mouse;
mod pointer;
