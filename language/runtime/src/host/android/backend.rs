use super::message as android_message;
use crate::diagnostic::RuntimeResult;
use crate::host::{HostBackend, Platform};

/// Android host implementation.
#[derive(Debug, Default)]
pub(crate) struct AndroidHost;

impl AndroidHost {
    /// Create one Android host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostBackend for AndroidHost {
    fn platform(&self) -> Platform {
        Platform::Android
    }

    fn process_native_ingress(&self) -> RuntimeResult<bool> {
        // service one ready slice of the Android looper
        Ok(android_message::process_ingress_ready(true))
    }
}
