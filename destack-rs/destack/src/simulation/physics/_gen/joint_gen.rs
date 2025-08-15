//! destack.simulation.physics.joint@2025.08.15.1

#![destack::generated(destack.simulation.physics.joint, file)]

use crate::JointBreakLimit;
use crate::JointConeLimit;
use crate::JointFlag;
use crate::JointFrame2D;
use crate::JointFrame3D;
use crate::JointMotor;
use crate::JointScalarLimit;
use crate::JointSpring;
use crate::JointTwistLimit;

#[destack::generated(JointSpring, Debug, block)]
impl std::fmt::Debug for JointSpring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JointSpring")
    }
}

#[destack::generated(JointScalarLimit, Debug, block)]
impl std::fmt::Debug for JointScalarLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JointScalarLimit")
    }
}

#[destack::generated(JointConeLimit, Debug, block)]
impl std::fmt::Debug for JointConeLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JointConeLimit")
    }
}

#[destack::generated(JointTwistLimit, Debug, block)]
impl std::fmt::Debug for JointTwistLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JointTwistLimit")
    }
}

#[destack::generated(JointBreakLimit, Debug, block)]
impl std::fmt::Debug for JointBreakLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JointBreakLimit")
    }
}

#[destack::generated(JointMotor, Debug, block)]
impl std::fmt::Debug for JointMotor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JointMotor")
    }
}

#[destack::generated(JointFrame2D, Debug, block)]
impl std::fmt::Debug for JointFrame2D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JointFrame2D")
    }
}

#[destack::generated(JointFrame3D, Debug, block)]
impl std::fmt::Debug for JointFrame3D {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JointFrame3D")
    }
}

#[destack::generated(JointFlag, Debug, block)]
impl std::fmt::Debug for JointFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JointFlag::Default => write!(f, "DEFAULT"),
            JointFlag::CollideConnected => write!(f, "COLLIDE_CONNECTED"),
        }
    }
}
