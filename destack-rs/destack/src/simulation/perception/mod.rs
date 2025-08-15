//! simulation/perception@2025.08.15.1

#![destack::partial(simulation/perception, file)]

pub use pointer::*;
pub use mouse::*;
pub use drag::*;
pub use key::*;
pub use input::*;
pub use focus::*;
pub use clipboard::*;

mod pointer;
mod mouse;
mod drag;
mod key;
mod input;
mod focus;
mod clipboard;