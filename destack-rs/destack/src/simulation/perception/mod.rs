//! destack.simulation.perception

#![destack::partial(destack.simulation.perception, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::simulation::perception::mouse::MouseButton;

pub mod _gen;
pub mod clipboard;
pub mod drag;
pub mod focus;
pub mod input;
pub mod key;
pub mod mouse;
pub mod pointer;
