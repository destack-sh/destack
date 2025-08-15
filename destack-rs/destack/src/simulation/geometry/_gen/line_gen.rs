//! destack.simulation.geometry.line@2025.08.15.1

#![destack::generated(destack.simulation.geometry.line, file)]

use crate::Segment2D;
use crate::Segment3D;

#[destack::generated(Segment2D, Debug, block)]
impl std::fmt::Debug for Segment2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Segment2D")
    }
}

#[destack::generated(Segment3D, Debug, block)]
impl std::fmt::Debug for Segment3D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Segment3D")
    }
}
