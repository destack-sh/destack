//! destack.simulation.geometry.line

#![destack::generated(destack.simulation.geometry.line, file)]

use crate::{Segment2D, Segment3D};

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
