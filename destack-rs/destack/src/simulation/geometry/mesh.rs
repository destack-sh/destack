//! destack.simulation.geometry.mesh

#![destack::partial(destack.simulation.geometry.mesh, file)]

use crate::{Vector2, Vector3};

#[destack::generated(Mesh2, -, block)]
/// A Mesh2D is defined by vertices and triangle indices.
pub struct Mesh2 {
    pub vertices: Vec<Vector2>,
    pub triangles: Vec<u32>,
}

#[destack::generated(Mesh3, -, block)]
/// A Mesh3D is defined by vertices and triangle indices.
pub struct Mesh3 {
    pub vertices: Vec<Vector3>,
    pub triangles: Vec<u32>,
}
