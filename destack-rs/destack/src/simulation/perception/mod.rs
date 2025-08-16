//! destack.simulation.perception@2025.08.15.1

#![destack::partial(destack.simulation.perception, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::simulation::perception::_gen::*;
pub use crate::simulation::perception::clipboard::*;
pub use crate::simulation::perception::drag::*;
pub use crate::simulation::perception::focus::*;
pub use crate::simulation::perception::input::*;
pub use crate::simulation::perception::key::*;
pub use crate::simulation::perception::mouse::*;
pub use crate::simulation::perception::pointer::*;

pub mod _gen;
pub mod clipboard;
pub mod drag;
pub mod focus;
pub mod input;
pub mod key;
pub mod mouse;
pub mod pointer;
