//! destack.simulation.geometry.quaternion

#![destack::partial(destack.simulation.geometry.quaternion, file)]

#[destack::generated(Quaternion, -, block)]
/// A quaternion.
pub struct Quaternion {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}
