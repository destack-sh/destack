//! destack.simulation.geometry.cone@2025.08.15.1

#![destack::partial(destack.simulation.geometry.cone, file)]

use crate::Vector3;

#[destack::generated(Cone3D, -, block)]
/// A Cone is aligned with the local z axis with a base radius and height.
pub struct Cone3D {
    pub position: Vector3,
    pub radius: f32,
    pub height: f32,
}
