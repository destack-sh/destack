use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Simulation state for runtime-backed and OS-backed simulation worlds.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimulationState {
    /// Simulation schema version.
    pub version: u32,
    /// Simulation clock subsystem state.
    pub clock: SimulationClockState,
    /// Simulation random subsystem state.
    pub random: SimulationRandomState,
    /// Simulation event loop subsystem state.
    pub event_loop: SimulationEventLoopState,
    /// Simulation filesystem subsystem state.
    pub fs: SimulationFsState,
    /// Simulation network subsystem state.
    pub net: SimulationNetState,
    /// Simulation process subsystem state.
    pub process: SimulationProcessState,
}

/// Simulation clock subsystem state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimulationClockState {}

/// Simulation random subsystem state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimulationRandomState {}

/// Simulation event loop subsystem state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimulationEventLoopState {}

/// Simulation filesystem subsystem state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimulationFsState {}

/// Simulation network subsystem state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimulationNetState {}

/// Simulation process subsystem state.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SimulationProcessState {}

/// Shared simulation state container.
pub type SharedSimulationState = RwLock<SimulationState>;
