use crate::diagnostic::RuntimeResult;
use crate::host::core::error::not_supported;
use crate::host::core::registry::HostSessionId;
use crate::host::core::request::{HostRequest, HostRequestOutcome, RequestContext, SessionContext};
use crate::host::{HostEvent, Platform};
use crate::runtime::action::{HostAction, HostActionSet};

/// Host poll output containing queued events and queue-pressure drops.
#[derive(Debug, Default)]
pub struct PollResult {
    /// Host events drained by one poll call.
    pub events: Vec<HostEvent>,
    /// Host events dropped by queue policy since the previous poll call.
    pub dropped_event_count: u64,
}

/// Shared process-global host adapter contract for one host integration family.
///
/// This is the runtime-facing host boundary above request transport and platform glue.
pub(crate) trait HostAdapter: std::fmt::Debug + Send + Sync {
    /// Return the host platform for this host adapter.
    fn platform(&self) -> Platform;

    /// Return static host actions for this host adapter target.
    fn static_actions(&self) -> HostActionSet {
        let mut actions = HostActionSet::new();

        // power state is a broadly available host binding family
        actions.insert_action(HostAction::OsPower);

        actions
    }

    /// Return dynamic session actions for one attached runtime session.
    fn session_actions(&self, _host_runtime_id: HostSessionId) -> HostActionSet {
        HostActionSet::new()
    }

    /// Return whether the current execution context is the process main context.
    fn is_process_main_context(&self) -> bool {
        false
    }

    /// Advance immediately ready native ingress without blocking.
    fn advance_native_ingress(&self) -> RuntimeResult<()> {
        Ok(())
    }

    /// Advance session-owned host ingress for one attached session.
    fn advance_session_ingress(&self, _context: &SessionContext) -> RuntimeResult<()> {
        Ok(())
    }

    /// Submit one runtime-owned host request through this host adapter.
    fn submit_request(
        &self,
        _context: &RequestContext,
        request: HostRequest,
    ) -> RuntimeResult<HostRequestOutcome> {
        Err(not_supported(request.operation_name()))
    }
}
