//! destack.simulation.geometry

#![destack::partial(destack.simulation.geometry, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::simulation::geometry::capsule::{Capsule2D, Capsule3D};
pub use crate::simulation::geometry::circle::{Circle2D, Sphere3D};
pub use crate::simulation::geometry::cone::Cone3D;
pub use crate::simulation::geometry::cylinder::Cylinder3D;
pub use crate::simulation::geometry::ellipse::{Ellipse2D, Ellipsoid3D};
pub use crate::simulation::geometry::line::{Segment2D, Segment3D};
pub use crate::simulation::geometry::mesh::{Mesh2, Mesh3};
pub use crate::simulation::geometry::path::{Path2D, Polyline3D};
pub use crate::simulation::geometry::plane::{Halfspace2D, Plane3D};
pub use crate::simulation::geometry::point::{Point2D, Point3D};
pub use crate::simulation::geometry::quaternion::Quaternion;
pub use crate::simulation::geometry::rectangle::{Box3D, Rectangle2D};
pub use crate::simulation::geometry::vector::{
    Vector2, Vector2i, Vector3, Vector3i, Vector4, Vector4i,
};

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
