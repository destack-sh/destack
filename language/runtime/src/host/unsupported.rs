use crate::host::{HostAdapter, Platform};
use crate::runtime::action::HostActionSet;

/// Unsupported host implementation.
#[derive(Debug, Default)]
pub(crate) struct UnsupportedHost;

impl UnsupportedHost {
    /// Create one unsupported host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostAdapter for UnsupportedHost {
    fn platform(&self) -> Platform {
        Platform::Unknown
    }

    fn static_actions(&self) -> HostActionSet {
        HostActionSet::new()
    }
}
