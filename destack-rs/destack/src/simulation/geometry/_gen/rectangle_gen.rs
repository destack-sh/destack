//! destack.simulation.geometry.rectangle

#![destack::generated(destack.simulation.geometry.rectangle, file)]

use crate::{Box3D, Rectangle2D};

#[destack::generated(Rectangle2D, Debug, block)]
impl std::fmt::Debug for Rectangle2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rectangle2D")
    }
}

#[destack::generated(Box3D, Debug, block)]
impl std::fmt::Debug for Box3D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Box3D")
    }
}
