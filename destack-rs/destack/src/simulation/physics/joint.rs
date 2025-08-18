//! destack.simulation.physics.joint

#![destack::partial(destack.simulation.physics.joint, file)]

use crate::{Quaternion, Vector2, Vector3};

#[destack::generated(JointSpring, -, block)]
/// Spring parameters for soft constraints.
/// Converts to ERP/CFM internally per constraint row. Higher stiffness makes the
/// constraint harder; higher damping reduces oscillation.
pub struct JointSpring {
    pub stiffness: f32,
    pub damping: f32,
}

#[destack::generated(JointScalarLimit, -, block)]
/// Scalar (1D) limit for linear or angular coordinates.
/// Applies to a single degree of freedom (e.g., a prismatic axis or a hinge twist).
/// Uses a contact distance to pre-activate before hitting the hard bound.
pub struct JointScalarLimit {
    pub min_value: f32,
    pub max_value: f32,
    pub restitution: f32,
    pub contact_distance: f32,
}

#[destack::generated(JointConeLimit, -, block)]
/// Elliptical swing (cone) limit for a ball-and-socket style joint.
/// Limits the swing of the relative orientation inside an elliptical cone defined
/// by maximum angles about the local Y and Z axes.
pub struct JointConeLimit {
    pub swing_y: f32,
    pub swing_z: f32,
    pub restitution: f32,
    pub contact_distance: f32,
}

#[destack::generated(JointTwistLimit, -, block)]
/// Angular 1D twist limit around a defined axis.
/// Like a scalar limit but with wrap-aware angle extraction around the twist axis.
pub struct JointTwistLimit {
    pub min_angle: f32,
    pub max_angle: f32,
    pub restitution: f32,
    pub contact_distance: f32,
}

#[destack::generated(JointBreakLimit, -, block)]
/// Break thresholds for a Joint.
pub struct JointBreakLimit {
    pub force: f32,
    pub torque: f32,
}

#[destack::generated(JointMotor, -, block)]
/// Motor/servo constraint for a Joint DoF.
/// Drives a linear or angular degree of freedom toward a target velocity or position.
/// A per-step impulse cap limits the work performed by the motor.
pub struct JointMotor {
    pub target_velocity: f32,
    pub max_impulse: f32,
    pub target_position: Option<f32>,
    pub stiffness: Option<f32>,
    pub damping: Option<f32>,
}

#[destack::generated(JointFrame2D, -, block)]
/// Frame for a Joint in 2D space (relative to a Body).
/// Defines the local anchor point and orientation used by the Joint on a body.
pub struct JointFrame2D {
    pub position: Vector2,
    pub rotation: f32,
}

#[destack::generated(JointFrame3D, -, block)]
/// Frame for a Joint in 3D space (relative to a Body).
/// Defines the local anchor point and orientation used by the Joint on a body.
pub struct JointFrame3D {
    pub position: Vector3,
    pub rotation: Quaternion,
}

#[destack::generated(JointFlag, -, block)]
/// Flags that modify Joint behavior.
pub enum JointFlag {
    Default = 0,
    /// If enabled, the two bodies connected by the Joint are allowed to collide
    /// with each other. Otherwise, contacts between them are suppressed.
    CollideConnected = 1,
}
