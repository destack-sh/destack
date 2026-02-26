/// Host lifecycle state.
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

/// Host memory pressure state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostMemoryPressureLevel {
    /// Memory pressure is normal.
    Normal,
    /// Memory pressure is elevated.
    Warning,
    /// Memory pressure is critical.
    Critical,
}

/// Host thermal state.
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

/// Host power mode state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HostPowerMode {
    /// Normal power mode.
    Normal,
    /// Low power mode.
    LowPower,
}
