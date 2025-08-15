//! destack.simulation.geometry@2025.08.15.1

#![destack::partial(simulation/geometry, file)]
#![allow(unused_imports)]

pub use crate::simulation::geometry::capsule::*;
pub use crate::simulation::geometry::circle::*;
pub use crate::simulation::geometry::cone::*;
pub(crate) use crate::simulation::geometry::convex::*;
pub use crate::simulation::geometry::cylinder::*;
pub use crate::simulation::geometry::ellipse::*;
pub(crate) use crate::simulation::geometry::entity::*;
pub use crate::simulation::geometry::line::*;
pub use crate::simulation::geometry::mesh::*;
pub use crate::simulation::geometry::path::*;
pub use crate::simulation::geometry::plane::*;
pub use crate::simulation::geometry::point::*;
pub use crate::simulation::geometry::quaternion::*;
pub use crate::simulation::geometry::rectangle::*;
pub use crate::simulation::geometry::shape::*;
pub use crate::simulation::geometry::vector::*;

mod capsule;
mod circle;
mod cone;
mod convex;
mod cylinder;
mod ellipse;
mod entity;
mod line;
mod mesh;
mod path;
mod plane;
mod point;
mod quaternion;
mod rectangle;
mod shape;
mod vector;
