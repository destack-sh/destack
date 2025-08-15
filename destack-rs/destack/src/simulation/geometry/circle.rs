//! destack.simulation.geometry.circle@2025.08.15.1

#![destack::partial(destack.simulation.geometry.circle, file)]

use crate::{Vector3, Vector2};

#[destack::generated(Circle2D, struct, block)]
/// A Circle is centered at a point with a radius.
pub struct Circle2D {
    position: Vector2,
    radius: f32
}

#[destack::generated(Sphere3D, struct, block)]
/// A Sphere is centered at a point with a radius.
pub struct Sphere3D {
    position: Vector3,
    radius: f32
}