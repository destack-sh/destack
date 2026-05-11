use crate::diagnostic::RuntimeResult;
use crate::host::os::linux::{action, ingress, request};
use crate::host::{
    HostAdapter, HostRequest, HostRequestOutcome, HostSessionId, Platform, RequestContext,
    SessionContext,
};
use crate::runtime::action::HostActionSet;

/// Linux host implementation.
#[derive(Debug, Default)]
pub(crate) struct LinuxHost;

impl LinuxHost {
    /// Create one Linux host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostAdapter for LinuxHost {
    fn platform(&self) -> Platform {
        Platform::Linux
    }

    fn static_actions(&self) -> HostActionSet {
        action::static_actions()
    }

    fn session_actions(&self, _host_runtime_id: HostSessionId) -> HostActionSet {
        action::session_actions()
    }

    fn advance_session_ingress(&self, context: &SessionContext) -> RuntimeResult<()> {
        ingress::service_linux_ingress(context)
    }

    fn submit_request(
        &self,
        context: &RequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        request::submit_request(context, request)
    }
}
