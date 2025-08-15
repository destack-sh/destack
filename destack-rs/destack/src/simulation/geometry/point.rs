//! destack.simulation.geometry.point@2025.08.15.1

#![destack::partial(destack.simulation.geometry.point, file)]

use crate::{Vector3, Vector2};

#[destack::generated(Point2D, struct, block)]
/// A Point is a zero-area shape with an optional local offset.
pub struct Point2D {
    position: Vector2
}

#[destack::generated(Point3D, struct, block)]
/// A Point3D is a zero-volume shape with an optional local offset.
pub struct Point3D {
    position: Vector3
}