//! destack.simulation.geometry.rectangle@2025.08.15.1

#![destack::partial(destack.simulation.geometry.rectangle, file)]

use crate::Vector2;
use crate::Vector3;

#[destack::generated(Rectangle2D, , block)]
/// A Rectangle is a rectangle.
pub struct Rectangle2D {
    position: Vector2,
    width: f32,
    height: f32,
}

#[destack::generated(Box3D, , block)]
/// A Box is an axis-aligned box defined by width, height and depth.
pub struct Box3D {
    position: Vector3,
    width: f32,
    height: f32,
    depth: f32,
}
