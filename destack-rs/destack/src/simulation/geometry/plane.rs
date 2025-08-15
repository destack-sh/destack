//! destack.simulation.geometry.plane@2025.08.15.1

#![destack::partial(destack.simulation.geometry.plane, file)]

use crate::Vector2;
use crate::Vector3;

#[destack::generated(Halfspace2D, -, block)]
/// A Halfspace2D splits 2D space by a line with normal and distance from origin.
pub struct Halfspace2D {
    pub position: Vector2,
    pub normal: Vector2,
    pub distance: f32,
}

#[destack::generated(Plane3D, -, block)]
/// A Plane3D splits 3D space by a plane with normal and distance from origin.
pub struct Plane3D {
    pub position: Vector3,
    pub normal: Vector3,
    pub distance: f32,
}
