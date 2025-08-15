//! destack.simulation.physics.joint@2025.08.15.1

#![destack::generated(destack.simulation.physics.joint, file)]

#[destack::generated(JointFlag, Debug, block)]
impl std::fmt::Debug for JointFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JointFlag::Default => write!(f, "DEFAULT"),
            JointFlag::CollideConnected => write!(f, "COLLIDE_CONNECTED"),
        }
    }
}