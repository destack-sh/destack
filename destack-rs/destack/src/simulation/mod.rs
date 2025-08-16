//! destack.simulation@2025.08.15.1

#![destack::partial(destack.simulation, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::simulation::geometry::{
    Box3D, Capsule2D, Capsule3D, Circle2D, Cone3D, Cylinder3D, Ellipse2D, Ellipsoid3D, Halfspace2D,
    Mesh2, Mesh3, Path2D, Plane3D, Point2D, Point3D, Polyline3D, Quaternion, Rectangle2D,
    Segment2D, Segment3D, Sphere3D, Vector2, Vector2i, Vector3, Vector3i, Vector4, Vector4i,
};
pub use crate::simulation::perception::MouseButton;
pub use crate::simulation::physics::{
    JointBreakLimit, JointConeLimit, JointFlag, JointFrame2D, JointFrame3D, JointMotor,
    JointScalarLimit, JointSpring, JointTwistLimit, RigidMotionMode,
};

pub mod geometry;
pub mod perception;
pub mod physics;
