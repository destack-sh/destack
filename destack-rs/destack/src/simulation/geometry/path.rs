//! destack.simulation.geometry.path@2025.08.15.1

#![destack::partial(destack.simulation.geometry.path, file)]

use crate::{Vector2, Vector3};

#[destack::generated(Path2D, -, block)]
/// A Path is a polyline of multiple points.
pub struct Path2D {
    pub position: Vector2,
    pub points: Vec<Vector2>,
}

#[destack::generated(Polyline3D, -, block)]
/// A Polyline3D is a polygonal chain in 3D space.
pub struct Polyline3D {
    pub position: Vector3,
    pub points: Vec<Vector3>,
}
