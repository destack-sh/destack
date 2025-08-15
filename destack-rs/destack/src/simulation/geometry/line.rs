//! destack.simulation.geometry.line@2025.08.15.1

#![destack::partial(destack.simulation.geometry.line, file)]

use crate::Vector2;
use crate::Vector3;

#[destack::generated(Segment2D, -, block)]
/// A Segment is a line between two points.
pub struct Segment2D {
    pub position: Vector2,
    pub start_offset: Vector2,
    pub end_offset: Vector2,
}

#[destack::generated(Segment3D, -, block)]
/// A Segment3D is a line between two points in 3D space.
pub struct Segment3D {
    pub position: Vector3,
    pub start_offset: Vector3,
    pub end_offset: Vector3,
}
