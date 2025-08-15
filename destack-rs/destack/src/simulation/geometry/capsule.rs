//! destack.simulation.geometry.capsule@2025.08.15.1

#![destack::partial(destack.simulation.geometry.capsule, file)]

use crate::Vector2;
use crate::Vector3;

#[destack::generated(Capsule2D, , block)]
/// A Capsule2D is a segment with rounded ends with a common radius.
pub struct Capsule2D {
    position: Vector2,
    center_a_offset: Vector2,
    center_b_offset: Vector2,
    radius: f32,
}

#[destack::generated(Capsule3D, , block)]
/// A Capsule3D is a segment with rounded ends in 3D with a common radius.
pub struct Capsule3D {
    position: Vector3,
    center_a_offset: Vector3,
    center_b_offset: Vector3,
    radius: f32,
}
