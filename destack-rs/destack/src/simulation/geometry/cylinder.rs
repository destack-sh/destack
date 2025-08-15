//! destack.simulation.geometry.cylinder@2025.08.15.1

#![destack::partial(destack.simulation.geometry.cylinder, file)]

use crate::Vector3;

#[destack::generated(Cylinder3D, -, block)]
/// A Cylinder is aligned with the local z axis with a radius and height.
pub struct Cylinder3D {
    pub position: Vector3,
    pub radius: f32,
    pub height: f32,
}
