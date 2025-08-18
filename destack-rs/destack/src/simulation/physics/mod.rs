//! destack.simulation.physics

#![destack::partial(destack.simulation.physics, file)]
#![allow(unused_imports)]
#![allow(unreachable_pub)]

pub use crate::simulation::physics::joint::{
    JointBreakLimit, JointConeLimit, JointFlag, JointFrame2D, JointFrame3D, JointMotor,
    JointScalarLimit, JointSpring, JointTwistLimit,
};
pub use crate::simulation::physics::rigid::RigidMotionMode;

pub mod _gen;
pub mod body;
pub mod collider;
pub mod joint;
pub mod rigid;
pub mod soft;
