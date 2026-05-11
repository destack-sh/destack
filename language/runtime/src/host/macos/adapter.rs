use crate::diagnostic::RuntimeResult;
use crate::host::os::apple::ingress::r#loop as apple_ingress_loop;
use crate::host::os::macos::{action, ingress, request};
use crate::host::{
    HostAdapter, HostRequest, HostRequestOutcome, HostSessionId, Platform, RequestContext,
    SessionContext,
};
use crate::runtime::action::HostActionSet;

/// macOS host implementation.
#[derive(Debug, Default)]
pub(crate) struct MacosHost;

impl MacosHost {
    /// Create one macOS host.
    pub(crate) fn new() -> Self {
        Self
    }
}

impl HostAdapter for MacosHost {
    fn platform(&self) -> Platform {
        Platform::MacOS
    }

    fn static_actions(&self) -> HostActionSet {
        action::static_actions()
    }

    fn session_actions(&self, _host_runtime_id: HostSessionId) -> HostActionSet {
        request::request_actions()
    }

    fn is_process_main_context(&self) -> bool {
        apple_ingress_loop::is_process_main_context()
    }

    fn advance_native_ingress(&self) -> RuntimeResult<()> {
        // drain one ready slice of the Apple run loop
        apple_ingress_loop::drain_ready_ingress();

        Ok(())
    }

    fn advance_session_ingress(&self, context: &SessionContext) -> RuntimeResult<()> {
        ingress::service_macos_ingress(context)
    }

    fn submit_request(
        &self,
        context: &RequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        request::submit_request(context, request)
    }
}
