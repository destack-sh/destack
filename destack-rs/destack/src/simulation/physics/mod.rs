//! simulation/physics@2025.08.15.1

#![destack::partial(simulation/physics, file)]

pub use soft::*;
pub use body::*;
pub use rigid::*;
pub use collider::*;
pub use joint::*;

mod soft;
mod body;
mod rigid;
mod collider;
mod joint;