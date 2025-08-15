//! destack.simulation.geometry.ellipse@2025.08.15.1

#![destack::partial(destack.simulation.geometry.ellipse, file)]

use crate::Vector2;
use crate::Vector3;

#[destack::generated(Ellipse2D, struct, block)]
/// An Ellipse is centered at a point with radii along x and y and a rotation.
pub struct Ellipse2D {
    position: Vector2,
    radius_x: f32,
    radius_y: f32,
    rotation: f32,
}

#[destack::generated(Ellipsoid3D, struct, block)]
/// An Ellipsoid is centered at a point with radii along x, y and z axes.
pub struct Ellipsoid3D {
    position: Vector3,
    radius_x: f32,
    radius_y: f32,
    radius_z: f32,
}
