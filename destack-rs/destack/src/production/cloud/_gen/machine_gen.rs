//! destack.production.cloud.machine@2025.08.15.1

#![destack::generated(destack.production.cloud.machine, file)]

use crate::MachineType;

#[destack::generated(MachineType, Debug, block)]
impl std::fmt::Debug for MachineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MachineType::Runtime => write!(f, "RUNTIME"),
            MachineType::Ubuntu => write!(f, "UBUNTU"),
            MachineType::Mac => write!(f, "MAC"),
            MachineType::Windows => write!(f, "WINDOWS"),
            MachineType::Custom => write!(f, "CUSTOM"),
        }
    }
}
