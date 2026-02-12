use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Simulation state for runtime-backed and OS-backed simulated worlds.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimulationState {
    /// Simulation schema version.
    pub version: u32,
    /// Simulated clock subsystem state.
    pub clock: SimulationClockState,
    /// Simulated random subsystem state.
    pub random: SimulationRandomState,
    /// Simulated scheduler subsystem state.
    pub scheduler: SimulationSchedulerState,
    /// Simulated filesystem subsystem state.
    pub fs: SimulationFsState,
    /// Simulated network subsystem state.
    pub net: SimulationNetState,
    /// Simulated process subsystem state.
    pub process: SimulationProcessState,
}

/// Simulated clock subsystem state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimulationClockState {}

/// Simulated random subsystem state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimulationRandomState {}

/// Simulated scheduler subsystem state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimulationSchedulerState {}

/// Simulated filesystem subsystem state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimulationFsState {}

/// Simulated network subsystem state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimulationNetState {}

/// Simulated process subsystem state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimulationProcessState {}

/// Shared simulation state container.
pub type SharedSimulationState = RwLock<SimulationState>;
