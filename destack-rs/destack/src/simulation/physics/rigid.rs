//! destack.simulation.physics.rigid@2025.08.15.1

#![destack::partial(destack.simulation.physics.rigid, file)]

#[destack::generated(RigidMotionMode, , block)]
/// RigidMotionMode
pub enum RigidMotionMode {
    /// Does not move or interact with the physics simulation.
    Static = 1,
    /// Interacts with the physics simulation but does not move by itself.
    Kinematic = 2,
    /// Interacts with the physics simulation and moves by itself.
    Dynamic = 3,
}
