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

#[destack::partial(destack.simulation.geometry.vector, vector2, block)]
/// Create a 2D floating point vector.
pub fn vector2(x: f32, y: f32) -> Vector2 {
    Vector2 { x, y }
}

#[destack::partial(destack.simulation.geometry.vector, vector3, block)]
/// Create a 3D floating point vector.
pub fn vector3(x: f32, y: f32, z: f32) -> Vector3 {
    Vector3 { x, y, z }
}

#[destack::partial(destack.simulation.geometry.vector, vector4, block)]
/// Create a 4D floating point vector.
pub fn vector4(x: f32, y: f32, z: f32, w: f32) -> Vector4 {
    Vector4 { x, y, z, w }
}

#[destack::partial(destack.simulation.geometry.vector, vector2i, block)]
/// Create a 2D integer vector.
pub fn vector2i(x: i32, y: i32) -> Vector2i {
    Vector2i { x, y }
}

#[destack::partial(destack.simulation.geometry.vector, vector3i, block)]
/// Create a 3D integer vector.
pub fn vector3i(x: i32, y: i32, z: i32) -> Vector3i {
    Vector3i { x, y, z }
}

#[destack::partial(destack.simulation.geometry.vector, vector4i, block)]
/// Create a 4D integer vector.
pub fn vector4i(x: i32, y: i32, z: i32, w: i32) -> Vector4i {
    Vector4i { x, y, z, w }
}
