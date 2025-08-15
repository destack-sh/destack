//! destack.simulation.geometry.circle@2025.08.15.1

#![destack::generated(destack.simulation.geometry.circle, file)]

use crate::Circle2D;
use crate::Sphere3D;

#[destack::generated(Circle2D, Debug, block)]
impl std::fmt::Debug for Circle2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Circle2D")
    }
}

#[destack::generated(Sphere3D, Debug, block)]
impl std::fmt::Debug for Sphere3D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sphere3D")
    }
}
