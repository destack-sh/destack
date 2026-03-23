use crate::diagnostic::RuntimeResult;
use crate::host::android::capability;
use crate::host::android::ingress::message as android_message;
use crate::host::android::request::submit_request;
use crate::host::core::error::not_supported;
use crate::host::core::{HostRequest, HostRequestContext, HostRequestOutcome, HostSessionId};
use crate::host::{HostAdapter, Platform};
use crate::runtime::capability::PlatformCapabilitySet;

/// Android host implementation.
#[derive(Debug, Default)]
pub(crate) struct AndroidHost;

impl AndroidHost {
    /// Create one Android host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostAdapter for AndroidHost {
    fn platform(&self) -> Platform {
        Platform::Android
    }

    fn static_capabilities(&self) -> PlatformCapabilitySet {
        capability::static_capabilities()
    }

    fn process_native_ingress(&self) -> RuntimeResult<()> {
        // service one ready slice of the Android looper
        android_message::process_ingress_ready(true);

        Ok(())
    }

    fn session_capabilities(&self, host_session_id: HostSessionId) -> PlatformCapabilitySet {
        capability::session_capabilities(host_session_id)
    }

    fn submit_request(
        &self,
        context: &HostRequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        if let Some(outcome) = submit_request(context, &request)? {
            return Ok(outcome);
        }

        Err(not_supported(request.operation_name()))
    }
}
