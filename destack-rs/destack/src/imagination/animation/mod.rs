//! imagination/animation@2025.08.15.1

#![destack::partial(imagination/animation, file)]

pub use transition::*;
pub use effect::*;
pub use easing::*;

mod transition;
mod effect;
mod easing;