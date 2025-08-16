//! destack.simulation.geometry.plane@2025.08.15.1

#![destack::generated(destack.simulation.geometry.plane, file)]

use crate::{Halfspace2D, Plane3D};

#[destack::generated(Halfspace2D, Debug, block)]
impl std::fmt::Debug for Halfspace2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Halfspace2D")
    }
}

#[destack::generated(Plane3D, Debug, block)]
impl std::fmt::Debug for Plane3D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Plane3D")
    }
}
