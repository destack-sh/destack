use std::sync::Arc;

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

/// Host runtime state integration surface.
pub trait HostStateReader: std::fmt::Debug + Send + Sync {
    /// Return the current lifecycle state.
    fn lifecycle_state(&self) -> HostLifecycleState;

    /// Return whether one native window is currently available.
    fn has_window(&self) -> bool;

    /// Return whether one native window is currently focused.
    fn is_window_focused(&self) -> bool;

    /// Return whether host interruption is currently active.
    fn is_interrupted(&self) -> bool;

    /// Return the current host memory pressure level.
    fn memory_pressure_level(&self) -> HostMemoryPressureLevel;

    /// Return the current host thermal state.
    fn thermal_state(&self) -> HostThermalState;

    /// Return the current host power mode.
    fn power_mode(&self) -> HostPowerMode;

    /// Return the number of host wall clock change events observed.
    fn wall_clock_change_count(&self) -> u64;
}

/// Host permission integration surface.
pub trait HostPermissionService: std::fmt::Debug + Send + Sync {
    /// Return whether the named permission currently has a pending request.
    fn is_request_in_flight(&self, permission: &str) -> bool;
}

/// Service surfaces exposed by one host adapter.
#[derive(Debug, Default, Clone)]
pub struct HostServices {
    /// Host state service when the host provides one.
    state: Option<Arc<dyn HostStateReader>>,
    /// Permission service when the host provides one.
    permission: Option<Arc<dyn HostPermissionService>>,
}

impl HostServices {
    /// Set the host state service.
    pub fn with_state(mut self, service: Arc<dyn HostStateReader>) -> Self {
        self.state = Some(service);
        self
    }

    /// Set the permission service.
    pub fn with_permission(mut self, service: Arc<dyn HostPermissionService>) -> Self {
        self.permission = Some(service);
        self
    }

    /// Return the host state service when available.
    pub fn state(&self) -> Option<&Arc<dyn HostStateReader>> {
        self.state.as_ref()
    }

    /// Return the permission service when available.
    pub fn permission(&self) -> Option<&Arc<dyn HostPermissionService>> {
        self.permission.as_ref()
    }
}
