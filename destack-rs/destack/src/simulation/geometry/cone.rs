//! destack.simulation.geometry.cone@2025.08.15.1

#![destack::partial(destack.simulation.geometry.cone, file)]

use crate::Vector3;

#[destack::generated(Cone3D, struct, block)]
/// A Cone is aligned with the local z axis with a base radius and height.
pub struct Cone3D {
    position: Vector3,
    radius: f32,
    height: f32,
}
