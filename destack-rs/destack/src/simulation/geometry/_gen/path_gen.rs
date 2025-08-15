//! destack.simulation.geometry.path@2025.08.15.1

#![destack::generated(destack.simulation.geometry.path, file)]

use crate::Path2D;
use crate::Polyline3D;

#[destack::generated(Path2D, Debug, block)]
impl std::fmt::Debug for Path2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Path2D")
    }
}

#[destack::generated(Polyline3D, Debug, block)]
impl std::fmt::Debug for Polyline3D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Polyline3D")
    }
}
