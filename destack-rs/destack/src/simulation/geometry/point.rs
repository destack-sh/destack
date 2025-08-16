//! destack.simulation.geometry.point@2025.08.15.1

#![destack::partial(destack.simulation.geometry.point, file)]

use crate::{Vector2, Vector3};

#[destack::generated(Point2D, -, block)]
/// A Point is a zero-area shape with an optional local offset.
pub struct Point2D {
    pub position: Vector2,
}

#[destack::generated(Point3D, -, block)]
/// A Point3D is a zero-volume shape with an optional local offset.
pub struct Point3D {
    pub position: Vector3,
}
