//! destack.simulation.geometry.ellipse

#![destack::partial(destack.simulation.geometry.ellipse, file)]

use crate::{Vector2, Vector3};

#[destack::generated(Ellipse2D, -, block)]
/// An Ellipse is centered at a point with radii along x and y and a rotation.
pub struct Ellipse2D {
    pub position: Vector2,
    pub radius_x: f32,
    pub radius_y: f32,
    pub rotation: f32,
}

#[destack::generated(Ellipsoid3D, -, block)]
/// An Ellipsoid is centered at a point with radii along x, y and z axes.
pub struct Ellipsoid3D {
    pub position: Vector3,
    pub radius_x: f32,
    pub radius_y: f32,
    pub radius_z: f32,
}
