//! destack.production.cloud.machine@2025.08.15.1

#![destack::partial(destack.production.cloud.machine, file)]

#[destack::generated(MachineType, enum, block)]
/// MachineType
pub enum MachineType {
    /// The main Destack runtime
    RUNTIME = 10,
    /// A Linux machine running Ubuntu
    UBUNTU = 1000,
    /// A Mac machine
    MAC = 1100,
    /// A Windows machine
    WINDOWS = 1200,
    /// A custom Docker image
    CUSTOM = 9000
}