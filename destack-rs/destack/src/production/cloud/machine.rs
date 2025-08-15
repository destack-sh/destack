//! destack.production.cloud.machine@2025.08.15.1

#![destack::partial(destack.production.cloud.machine, file)]

#[destack::generated(MachineType, enum, block)]
/// MachineType
pub enum MachineType {
    /// The main Destack runtime
    Runtime = 10,
    /// A Linux machine running Ubuntu
    Ubuntu = 1000,
    /// A Mac machine
    Mac = 1100,
    /// A Windows machine
    Windows = 1200,
    /// A custom Docker image
    Custom = 9000
}