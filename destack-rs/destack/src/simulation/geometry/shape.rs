//! destack.simulation.geometry.shape@2025.08.15.1

#![destack::partial(destack.simulation.geometry.shape, file)]

use crate::Vector2;
use crate::Vector3;

#[destack::generated(Form2D, struct, block)]
/// Represent 2-dimensional geometric shapes with position in the abstract.
pub struct Form2D {
    position: Vector2,
}

#[destack::generated(Form3D, struct, block)]
/// Represent 3-dimensional geometric shapes with position in the abstract.
pub struct Form3D {
    position: Vector3,
}
