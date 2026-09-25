use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;

/// Host lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum LifecycleState {
    /// Runtime has not received start events yet.
    Initializing,
    /// Runtime is active and can process host interactions.
    Running,
    /// Runtime is paused by the host.
    Paused,
    /// Runtime is stopped by the host.
    Stopped,
    /// Runtime host process is being destroyed.
    Destroyed,
}

/// Host lifecycle source attachment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum LifecycleSourceKind {
    /// Event originated from one application attachment.
    Application,
    /// Event originated from one worker attachment.
    Worker,
    /// Event originated from one window attachment.
    Window,
}

/// Host memory pressure state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum MemoryPressureLevel {
    /// Memory pressure is normal.
    Normal,
    /// Memory pressure is elevated.
    Warning,
    /// Memory pressure is critical.
    Critical,
}

/// Host thermal state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum ThermalState {
    /// Thermal state is nominal.
    Nominal,
    /// Thermal state is fair.
    Fair,
    /// Thermal state is serious.
    Serious,
    /// Thermal state is critical.
    Critical,
}

/// Host power mode state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum PowerMode {
    /// Normal power mode.
    Normal,
    /// Low power mode.
    LowPower,
}

/// Host event kind for queue policy.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub enum HostEventKind {
    /// Lifecycle transitions.
    Lifecycle,
    /// Memory pressure state events.
    MemoryPressure,
    /// Thermal state events.
    ThermalState,
    /// Power mode events.
    PowerMode,
    /// Wall clock change events.
    WallClock,
}

/// Runtime-visible host ingress payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum HostEvent {
    /// Host lifecycle transition event.
    Lifecycle(LifecycleEvent),
    /// Host memory pressure event.
    MemoryPressure(MemoryPressureEvent),
    /// Host thermal state event.
    ThermalState(ThermalEvent),
    /// Host power mode event.
    PowerMode(PowerModeEvent),
    /// Host wall clock change event.
    WallClock(WallClockEvent),
}

impl HostEvent {
    /// Return the semantic kind for this event.
    pub const fn kind(&self) -> HostEventKind {
        match self {
            HostEvent::Lifecycle(_) => HostEventKind::Lifecycle,
            HostEvent::MemoryPressure(_) => HostEventKind::MemoryPressure,
            HostEvent::ThermalState(_) => HostEventKind::ThermalState,
            HostEvent::PowerMode(_) => HostEventKind::PowerMode,
            HostEvent::WallClock(_) => HostEventKind::WallClock,
        }
    }
}

/// Host lifecycle transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct LifecycleEvent {
    /// Lifecycle source.
    pub source_kind: LifecycleSourceKind,
    /// Lifecycle state.
    pub state: LifecycleState,
}

/// Host memory pressure event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct MemoryPressureEvent {
    /// Memory pressure level.
    pub level: MemoryPressureLevel,
}

/// Host thermal state event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct ThermalEvent {
    /// Thermal state.
    pub state: ThermalState,
}

/// Host power mode event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct PowerModeEvent {
    /// Power mode.
    pub mode: PowerMode,
}

/// Host wall clock change event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct WallClockEvent;
