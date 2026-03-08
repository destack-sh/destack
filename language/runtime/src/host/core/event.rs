/// Host lifecycle state.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostLifecycleState {
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

impl HostLifecycleState {
    /// Return this lifecycle state as one compact atomic value.
    pub(crate) const fn encode(self) -> u8 {
        self as u8
    }

    /// Decode one compact atomic lifecycle value.
    #[cfg(test)]
    pub(crate) const fn decode(encoded_state: u8) -> Self {
        match encoded_state {
            1 => Self::Running,
            2 => Self::Paused,
            3 => Self::Stopped,
            4 => Self::Destroyed,
            _ => Self::Initializing,
        }
    }
}

/// Host memory pressure state.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostMemoryPressureLevel {
    /// Memory pressure is normal.
    Normal,
    /// Memory pressure is elevated.
    Warning,
    /// Memory pressure is critical.
    Critical,
}

impl HostMemoryPressureLevel {
    /// Return this memory pressure level as one compact atomic value.
    pub(crate) const fn encode(self) -> u8 {
        self as u8
    }

    /// Decode one compact atomic memory pressure value.
    #[cfg(test)]
    pub(crate) const fn decode(encoded_level: u8) -> Self {
        match encoded_level {
            1 => Self::Warning,
            2 => Self::Critical,
            _ => Self::Normal,
        }
    }
}

/// Host thermal state.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostThermalState {
    /// Thermal state is nominal.
    Nominal,
    /// Thermal state is fair.
    Fair,
    /// Thermal state is serious.
    Serious,
    /// Thermal state is critical.
    Critical,
}

impl HostThermalState {
    /// Return this thermal state as one compact atomic value.
    pub(crate) const fn encode(self) -> u8 {
        self as u8
    }

    /// Decode one compact atomic thermal value.
    #[cfg(test)]
    pub(crate) const fn decode(encoded_state: u8) -> Self {
        match encoded_state {
            1 => Self::Fair,
            2 => Self::Serious,
            3 => Self::Critical,
            _ => Self::Nominal,
        }
    }
}

/// Host power mode state.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostPowerMode {
    /// Normal power mode.
    Normal,
    /// Low power mode.
    LowPower,
}

impl HostPowerMode {
    /// Return this power mode as one compact atomic value.
    pub(crate) const fn encode(self) -> u8 {
        self as u8
    }

    /// Decode one compact atomic power mode value.
    #[cfg(test)]
    pub(crate) const fn decode(encoded_mode: u8) -> Self {
        match encoded_mode {
            1 => Self::LowPower,
            _ => Self::Normal,
        }
    }
}

/// Host semantic event kind key for scheduler watches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostEventKind {
    /// Lifecycle transitions.
    Lifecycle,
    /// Permission result events.
    Permission,
    /// Interruption events.
    Interruption,
    /// Memory pressure state events.
    MemoryPressure,
    /// Thermal state events.
    ThermalState,
    /// Power mode events.
    PowerMode,
    /// Wall clock change events.
    WallClock,
}

/// Runtime-visible host event payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostEvent {
    /// Host lifecycle transition event.
    Lifecycle(HostLifecycleEvent),
    /// Host permission flow result event.
    Permission(HostPermissionEvent),
    /// Host interruption event.
    Interruption(HostInterruptionEvent),
    /// Host memory pressure state event.
    MemoryPressure(HostMemoryPressureEvent),
    /// Host thermal state event.
    ThermalState(HostThermalEvent),
    /// Host power mode event.
    PowerMode(HostPowerModeEvent),
    /// Host wall clock change event.
    WallClock(HostWallClockEvent),
}

impl HostEvent {
    /// Return the host semantic event kind for this event.
    pub const fn kind(&self) -> HostEventKind {
        match self {
            HostEvent::Lifecycle(_) => HostEventKind::Lifecycle,
            HostEvent::Permission(_) => HostEventKind::Permission,
            HostEvent::Interruption(_) => HostEventKind::Interruption,
            HostEvent::MemoryPressure(_) => HostEventKind::MemoryPressure,
            HostEvent::ThermalState(_) => HostEventKind::ThermalState,
            HostEvent::PowerMode(_) => HostEventKind::PowerMode,
            HostEvent::WallClock(_) => HostEventKind::WallClock,
        }
    }
}

/// Host lifecycle state change payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostLifecycleEvent {
    /// Next lifecycle state after this transition.
    pub state: HostLifecycleState,
}

/// Host permission flow payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostPermissionEvent {
    /// Permission name associated with this result.
    pub permission: String,
    /// Whether the permission was granted.
    pub granted: bool,
}

/// Host interruption payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostInterruptionEvent {
    /// Whether the host is currently interrupted.
    pub interrupted: bool,
}

/// Host memory pressure payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostMemoryPressureEvent {
    /// Memory pressure level reported by the host.
    pub level: HostMemoryPressureLevel,
}

/// Host thermal payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostThermalEvent {
    /// Thermal state reported by the host.
    pub state: HostThermalState,
}

/// Host power mode payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostPowerModeEvent {
    /// Power mode reported by the host.
    pub mode: HostPowerMode,
}

/// Host wall clock payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostWallClockEvent;
