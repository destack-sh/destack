//! destack.simulation.geometry.circle

#![destack::partial(destack.simulation.geometry.circle, file)]

use crate::{Vector2, Vector3};

#[destack::generated(Circle2D, -, block)]
/// A Circle is centered at a point with a radius.
pub struct Circle2D {
    pub position: Vector2,
    pub radius: f32,
}

#[destack::generated(Sphere3D, -, block)]
/// A Sphere is centered at a point with a radius.
pub struct Sphere3D {
    pub position: Vector3,
    pub radius: f32,
}
