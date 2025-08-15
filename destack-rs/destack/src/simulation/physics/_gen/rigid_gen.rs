//! destack.simulation.physics.rigid@2025.08.15.1

#![destack::generated(destack.simulation.physics.rigid, file)]

use crate::RigidMotionMode;

#[destack::generated(RigidMotionMode, Debug, block)]
impl std::fmt::Debug for RigidMotionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RigidMotionMode::Static => write!(f, "STATIC"),
            RigidMotionMode::Kinematic => write!(f, "KINEMATIC"),
            RigidMotionMode::Dynamic => write!(f, "DYNAMIC"),
        }
    }
}
