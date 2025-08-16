//! destack.simulation.geometry@2025.08.15.1

#![destack::partial(destack.simulation.geometry, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::simulation::geometry::capsule::*;
pub use crate::simulation::geometry::circle::*;
pub use crate::simulation::geometry::cone::*;
pub use crate::simulation::geometry::convex::*;
pub use crate::simulation::geometry::cylinder::*;
pub use crate::simulation::geometry::ellipse::*;
pub use crate::simulation::geometry::entity::*;
pub use crate::simulation::geometry::line::*;
pub use crate::simulation::geometry::mesh::*;
pub use crate::simulation::geometry::path::*;
pub use crate::simulation::geometry::plane::*;
pub use crate::simulation::geometry::point::*;
pub use crate::simulation::geometry::quaternion::*;
pub use crate::simulation::geometry::rectangle::*;
pub use crate::simulation::geometry::shape::*;
pub use crate::simulation::geometry::vector::*;

pub mod _gen;
pub mod capsule;
pub mod circle;
pub mod cone;
pub mod convex;
pub mod cylinder;
pub mod ellipse;
pub mod entity;
pub mod line;
pub mod mesh;
pub mod path;
pub mod plane;
pub mod point;
pub mod quaternion;
pub mod rectangle;
pub mod shape;
pub mod vector;
