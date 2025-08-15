//! destack.simulation.geometry.vector@2025.08.15.1

#![destack::partial(destack.simulation.geometry.vector, file)]

#[destack::generated(Vector2, -, block)]
/// A 2D floating point Vector.
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}

#[destack::generated(Vector2i, -, block)]
/// A 2D integer Vector.
pub struct Vector2i {
    pub x: i32,
    pub y: i32,
}

#[destack::generated(Vector3, -, block)]
/// A 3D floating point Vector.
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[destack::generated(Vector3i, -, block)]
/// A 3D integer Vector.
pub struct Vector3i {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[destack::generated(Vector4, -, block)]
/// A 4D floating point Vector.
pub struct Vector4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

#[destack::generated(Vector4i, -, block)]
/// A 4D integer Vector.
pub struct Vector4i {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub w: i32,
}
