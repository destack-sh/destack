//! destack.simulation.geometry.mesh@2025.08.15.1

#![destack::partial(destack.simulation.geometry.mesh, file)]

use crate::Vector2;
use crate::Vector3;

#[destack::generated(Mesh2, struct, block)]
/// A Mesh2D is defined by vertices and triangle indices.
pub struct Mesh2 {
    vertices: Vec<Vector2>,
    triangles: Vec<u32>,
}

#[destack::generated(Mesh3, struct, block)]
/// A Mesh3D is defined by vertices and triangle indices.
pub struct Mesh3 {
    vertices: Vec<Vector3>,
    triangles: Vec<u32>,
}
