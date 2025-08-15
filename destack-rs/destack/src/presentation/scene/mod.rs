//! presentation/scene@2025.08.15.1

#![destack::partial(presentation/scene, file)]

pub use stage::*;
pub use layer::*;
pub use scene::*;

mod stage;
mod layer;
mod scene;