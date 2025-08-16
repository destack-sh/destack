//! destack.simulation.geometry.rectangle@2025.08.15.1

#![destack::partial(destack.simulation.geometry.rectangle, file)]

use crate::{Vector2, Vector3};

#[destack::generated(Rectangle2D, -, block)]
/// A Rectangle is a rectangle.
pub struct Rectangle2D {
    pub position: Vector2,
    pub width: f32,
    pub height: f32,
}

#[destack::generated(Box3D, -, block)]
/// A Box is an axis-aligned box defined by width, height and depth.
pub struct Box3D {
    pub position: Vector3,
    pub width: f32,
    pub height: f32,
    pub depth: f32,
}
