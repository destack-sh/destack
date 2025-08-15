//! destack.simulation.geometry.capsule@2025.08.15.1

#![destack::generated(destack.simulation.geometry.capsule, file)]

use crate::Capsule2D;
use crate::Capsule3D;

#[destack::generated(Capsule2D, Debug, block)]
impl std::fmt::Debug for Capsule2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Capsule2D")
    }
}

#[destack::generated(Capsule3D, Debug, block)]
impl std::fmt::Debug for Capsule3D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Capsule3D")
    }
}
