use crate::diagnostic::RuntimeResult;
use crate::host::{HostEvent, Platform};
use crate::world::policy::ActionSet;

/// Host poll output containing queued events.
#[derive(Debug, Default)]
pub struct HostPollResult {
    /// Host events drained by one poll call.
    pub events: Vec<HostEvent>,
}

/// Runtime boundary for process-local host integration.
pub(crate) trait Host: std::fmt::Debug + Send + Sync {
    /// Return the target platform for this host.
    fn platform(&self) -> Platform;

    /// Return static host actions implemented by this host.
    fn static_actions(&self) -> ActionSet {
        ActionSet::new()
    }

    /// Return whether the current execution context is the process main context.
    fn is_process_main_context(&self) -> bool {
        false
    }

    /// Advance immediately ready host events without blocking.
    fn advance_events(&self) -> RuntimeResult<()> {
        Ok(())
    }

    /// Collect session-owned host ingress for one attached session.
    fn collect_session_events(&self) -> RuntimeResult<Vec<HostEvent>> {
        Ok(Vec::new())
    }
}

/// Fallback host integration for unsupported compile targets.
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
#[derive(Debug, Default)]
pub(crate) struct UnsupportedHost;

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
impl UnsupportedHost {
    /// Create one fallback host integration.
    pub(crate) fn new() -> Self {
        Self
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
impl Host for UnsupportedHost {
    fn platform(&self) -> Platform {
        Platform::Unknown
    }
}
