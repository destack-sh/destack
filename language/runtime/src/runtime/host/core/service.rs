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

/// Host lifecycle integration surface.
pub trait HostLifecycleService: std::fmt::Debug + Send + Sync {
    /// Return the current lifecycle state.
    fn state(&self) -> HostLifecycleState;
}

/// Host window integration surface.
pub trait HostWindowService: std::fmt::Debug + Send + Sync {
    /// Return whether one native window is currently available.
    fn has_window(&self) -> bool;
}

/// Host permission integration surface.
pub trait HostPermissionService: std::fmt::Debug + Send + Sync {
    /// Return whether the named permission currently has a pending request.
    fn is_request_in_flight(&self, permission: &str) -> bool;
}

/// Host interruption integration surface.
pub trait HostInterruptionService: std::fmt::Debug + Send + Sync {
    /// Return whether host interruption is currently active.
    fn is_interrupted(&self) -> bool;
}

/// Service surfaces exposed by one host adapter.
#[derive(Debug, Default, Clone)]
pub struct HostServices {
    /// Lifecycle service when the host provides one.
    lifecycle: Option<Arc<dyn HostLifecycleService>>,
    /// Window service when the host provides one.
    window: Option<Arc<dyn HostWindowService>>,
    /// Permission service when the host provides one.
    permission: Option<Arc<dyn HostPermissionService>>,
    /// Interruption service when the host provides one.
    interruption: Option<Arc<dyn HostInterruptionService>>,
}

impl HostServices {
    /// Set the lifecycle service.
    pub fn with_lifecycle(mut self, service: Arc<dyn HostLifecycleService>) -> Self {
        self.lifecycle = Some(service);
        self
    }

    /// Set the window service.
    pub fn with_window(mut self, service: Arc<dyn HostWindowService>) -> Self {
        self.window = Some(service);
        self
    }

    /// Set the permission service.
    pub fn with_permission(mut self, service: Arc<dyn HostPermissionService>) -> Self {
        self.permission = Some(service);
        self
    }

    /// Set the interruption service.
    pub fn with_interruption(mut self, service: Arc<dyn HostInterruptionService>) -> Self {
        self.interruption = Some(service);
        self
    }

    /// Return the lifecycle service when available.
    pub fn lifecycle(&self) -> Option<&Arc<dyn HostLifecycleService>> {
        self.lifecycle.as_ref()
    }

    /// Return the window service when available.
    pub fn window(&self) -> Option<&Arc<dyn HostWindowService>> {
        self.window.as_ref()
    }

    /// Return the permission service when available.
    pub fn permission(&self) -> Option<&Arc<dyn HostPermissionService>> {
        self.permission.as_ref()
    }

    /// Return the interruption service when available.
    pub fn interruption(&self) -> Option<&Arc<dyn HostInterruptionService>> {
        self.interruption.as_ref()
    }
}
