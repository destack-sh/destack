//! destack.simulation.physics.joint@2025.08.14.0

#![destack::partial(destack.simulation.physics.joint, file)]

#[destack::generated(JointSpring, struct, block)]
/// Spring parameters for soft constraints.
/// Converts to ERP/CFM internally per constraint row. Higher stiffness makes the
/// constraint harder; higher damping reduces oscillation.
pub struct JointSpring {

}

#[destack::generated(JointScalarLimit, struct, block)]
/// Scalar (1D) limit for linear or angular coordinates.
/// Applies to a single degree of freedom (e.g., a prismatic axis or a hinge twist).
/// Uses a contact distance to pre-activate before hitting the hard bound.
pub struct JointScalarLimit {

}

#[destack::generated(JointConeLimit, struct, block)]
/// Elliptical swing (cone) limit for a ball-and-socket style joint.
/// Limits the swing of the relative orientation inside an elliptical cone defined
/// by maximum angles about the local Y and Z axes.
pub struct JointConeLimit {

}

#[destack::generated(JointTwistLimit, struct, block)]
/// Angular 1D twist limit around a defined axis.
/// Like a scalar limit but with wrap-aware angle extraction around the twist axis.
pub struct JointTwistLimit {

}

#[destack::generated(JointBreakLimit, struct, block)]
/// Break thresholds for a Joint.
pub struct JointBreakLimit {

}

#[destack::generated(JointMotor, struct, block)]
/// Motor/servo constraint for a Joint DoF.
/// Drives a linear or angular degree of freedom toward a target velocity or position.
/// A per-step impulse cap limits the work performed by the motor.
pub struct JointMotor {

}

#[destack::generated(JointFrame2D, struct, block)]
/// Frame for a Joint in 2D space (relative to a Body).
/// Defines the local anchor point and orientation used by the Joint on a body.
pub struct JointFrame2D {

}

#[destack::generated(JointFrame3D, struct, block)]
/// Frame for a Joint in 3D space (relative to a Body).
/// Defines the local anchor point and orientation used by the Joint on a body.
pub struct JointFrame3D {

}

#[destack::generated(JointFlag, enum, block)]
/// Flags that modify Joint behavior.
pub enum JointFlag {
    DEFAULT = 0,
    /// If enabled, the two bodies connected by the Joint are allowed to collide
    with each other. Otherwise, contacts between them are suppressed.

    COLLIDE_CONNECTED = 1
}