//! destack.simulation.geometry.ellipse@2025.08.15.1

#![destack::generated(destack.simulation.geometry.ellipse, file)]

use crate::Ellipse2D;
use crate::Ellipsoid3D;

#[destack::generated(Ellipse2D, Debug, block)]
impl std::fmt::Debug for Ellipse2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ellipse2D")
    }
}

#[destack::generated(Ellipsoid3D, Debug, block)]
impl std::fmt::Debug for Ellipsoid3D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ellipsoid3D")
    }
}
