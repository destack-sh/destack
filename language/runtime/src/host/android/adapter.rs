use crate::diagnostic::RuntimeResult;
use crate::host::core::error::not_supported;
use crate::host::os::android::action;
use crate::host::os::android::ingress::r#loop as android_ingress_loop;
use crate::host::os::android::request::submit_request;
use crate::host::{
    HostAdapter, HostRequest, HostRequestOutcome, HostSessionId, Platform, RequestContext,
};
use crate::runtime::action::HostActionSet;

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

    fn static_actions(&self) -> HostActionSet {
        action::static_actions()
    }

    fn advance_native_ingress(&self) -> RuntimeResult<()> {
        // drain one ready slice of the Android ingress loop
        android_ingress_loop::drain_ready_ingress();

        Ok(())
    }

    fn session_actions(&self, host_session_id: HostSessionId) -> HostActionSet {
        action::session_actions(host_session_id)
    }

    fn submit_request(
        &self,
        context: &RequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        if let Some(outcome) = submit_request(context, &request)? {
            return Ok(outcome);
        }

        Err(not_supported(request.operation_name()))
    }
}
