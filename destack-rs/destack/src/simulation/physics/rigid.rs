//! destack.simulation.physics.rigid@2025.08.15.1

#![destack::partial(destack.simulation.physics.rigid, file)]

#[destack::generated(RigidMotionMode, enum, block)]
/// RigidMotionMode
pub enum RigidMotionMode {
    /// Does not move or interact with the physics simulation.
    STATIC = 1,
    /// Interacts with the physics simulation but does not move by itself.
    KINEMATIC = 2,
    /// Interacts with the physics simulation and moves by itself.
    DYNAMIC = 3
}