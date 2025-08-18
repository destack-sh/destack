//! destack.simulation.geometry.vector

#![destack::generated(destack.simulation.geometry.vector, file)]

use crate::{Vector2, Vector2i, Vector3, Vector3i, Vector4, Vector4i};

#[destack::generated(Vector2, Debug, block)]
impl std::fmt::Debug for Vector2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Vector2")
    }
}

#[destack::generated(Vector2i, Debug, block)]
impl std::fmt::Debug for Vector2i {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Vector2i")
    }
}

#[destack::generated(Vector3, Debug, block)]
impl std::fmt::Debug for Vector3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Vector3")
    }
}

#[destack::generated(Vector3i, Debug, block)]
impl std::fmt::Debug for Vector3i {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Vector3i")
    }
}

#[destack::generated(Vector4, Debug, block)]
impl std::fmt::Debug for Vector4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Vector4")
    }
}

#[destack::generated(Vector4i, Debug, block)]
impl std::fmt::Debug for Vector4i {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Vector4i")
    }
}
