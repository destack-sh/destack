//! destack.simulation.geometry@2025.08.15.1

#![destack::partial(simulation/geometry, file)]
#![allow(unused_imports)]

pub use crate::simulation::geometry::mesh::*;
pub use crate::simulation::geometry::convex::*;
pub use crate::simulation::geometry::entity::*;
pub use crate::simulation::geometry::cylinder::*;
pub use crate::simulation::geometry::capsule::*;
pub use crate::simulation::geometry::quaternion::*;
pub use crate::simulation::geometry::cone::*;
pub use crate::simulation::geometry::point::*;
pub use crate::simulation::geometry::ellipse::*;
pub use crate::simulation::geometry::path::*;
pub use crate::simulation::geometry::shape::*;
pub use crate::simulation::geometry::line::*;
pub use crate::simulation::geometry::circle::*;
pub use crate::simulation::geometry::plane::*;
pub use crate::simulation::geometry::vector::*;
pub use crate::simulation::geometry::rectangle::*;

mod mesh;
mod convex;
mod entity;
mod cylinder;
mod capsule;
mod quaternion;
mod cone;
mod point;
mod ellipse;
mod path;
mod shape;
mod line;
mod circle;
mod plane;
mod vector;
mod rectangle;

pub(crate) use crate::simulation::geometry::convex::*;

pub(crate) use crate::simulation::geometry::entity::*;

pub(crate) use crate::simulation::geometry::convex::*;

pub(crate) use crate::simulation::geometry::entity::*;

pub(crate) use crate::simulation::geometry::convex::*;

pub(crate) use crate::simulation::geometry::entity::*;