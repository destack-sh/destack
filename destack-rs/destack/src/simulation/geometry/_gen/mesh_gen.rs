//! destack.simulation.geometry.mesh

#![destack::generated(destack.simulation.geometry.mesh, file)]

use crate::{Mesh2, Mesh3};

#[destack::generated(Mesh2, Debug, block)]
impl std::fmt::Debug for Mesh2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mesh2")
    }
}

#[destack::generated(Mesh3, Debug, block)]
impl std::fmt::Debug for Mesh3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Mesh3")
    }
}
