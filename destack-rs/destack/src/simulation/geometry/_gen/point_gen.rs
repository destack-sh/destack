//! destack.simulation.geometry.point@2025.08.15.1

#![destack::generated(destack.simulation.geometry.point, file)]

use crate::{Point2D, Point3D};

#[destack::generated(Point2D, Debug, block)]
impl std::fmt::Debug for Point2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Point2D")
    }
}

#[destack::generated(Point3D, Debug, block)]
impl std::fmt::Debug for Point3D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Point3D")
    }
}
