//! destack.simulation.geometry.path@2025.08.15.1

#![destack::partial(destack.simulation.geometry.path, file)]

use crate::Vector2;
use crate::Vector3;

#[destack::generated(Path2D, struct, block)]
/// A Path is a polyline of multiple points.
pub struct Path2D {
    position: Vector2,
    points: Vec<Vector2>,
}

#[destack::generated(Polyline3D, struct, block)]
/// A Polyline3D is a polygonal chain in 3D space.
pub struct Polyline3D {
    position: Vector3,
    points: Vec<Vector3>,
}
