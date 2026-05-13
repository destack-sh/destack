use std::sync::Arc;

use destack_artifact::Platform;

use crate::diagnostic::RuntimeResult;
use crate::host::core::host::{Host, HostPollResult};
use crate::host::core::queue::HostQueue;
use crate::host::core::target::default_compile_target_host;
use crate::host::poller::PollerWakeHandle;
use crate::world::RuntimeId;
use crate::world::policy::{Action, ActionId, ActionSet};

/// Runtime-scoped connection to host integration.
pub struct HostSession {
    /// Active host integration for this runtime session.
    host: Arc<dyn Host>,
    /// Shared ingress queue for this runtime instance.
    queue: Arc<HostQueue>,
    /// Logical runtime id used by the world/runtime layer.
    runtime_id: RuntimeId,
    /// Static action set implemented by the host.
    host_actions: ActionSet,
}

impl std::fmt::Debug for HostSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostSession")
            .field("target_platform", &self.platform())
            .field("runtime_id", &self.runtime_id)
            .field("host_action_count", &self.host_actions.len())
            .finish()
    }
}

impl HostSession {
    /// Create one session from one explicit host integration.
    pub(crate) fn new(host: Arc<dyn Host>, runtime_id: RuntimeId) -> Self {
        let queue = Arc::new(HostQueue::new());
        let host_actions = host.static_actions();

        Self {
            host,
            queue,
            runtime_id,
            host_actions,
        }
    }

    /// Create one session from one explicit runtime id.
    pub(crate) fn from_runtime_id(runtime_id: RuntimeId) -> Self {
        let host = default_compile_target_host();

        Self::new(host, runtime_id)
    }

    /// Return the target platform.
    pub fn platform(&self) -> Platform {
        self.host.platform()
    }

    /// Return effective host actions implemented by host and session wiring.
    pub fn host_actions(&self) -> ActionSet {
        self.host_actions.clone()
    }

    /// Return the static actions implemented by this host.
    pub fn static_actions(&self) -> &ActionSet {
        &self.host_actions
    }

    /// Return whether the host and session wiring implement one host action id.
    pub fn has_host_action_id(&self, action_id: ActionId) -> bool {
        self.host_actions.contains_id(action_id)
    }

    /// Return whether the host and session wiring implement one host action.
    pub fn has_host_action(&self, action: Action) -> bool {
        self.has_host_action_id(action.id())
    }

    /// Poll host events using the active session.
    pub fn poll(&self, timeout_nanos: Option<u64>) -> RuntimeResult<HostPollResult> {
        self.advance_ingress()?;

        Ok(HostPollResult {
            events: self.queue.poll_events(timeout_nanos)?,
        })
    }

    /// Return one shared host wake handle.
    pub fn poll_wake_handle(&self) -> Arc<dyn PollerWakeHandle> {
        self.queue.poll_wake_handle()
    }

    /// Return the logical runtime id for this session.
    pub const fn runtime_id(&self) -> RuntimeId {
        self.runtime_id
    }

    /// Return whether the current execution context is the process main context.
    pub fn is_process_main_context(&self) -> bool {
        self.host.is_process_main_context()
    }

    /// Advance host-owned and session-owned events for this session.
    pub(crate) fn advance_ingress(&self) -> RuntimeResult<()> {
        self.host.advance_events()?;
        let events = self.host.collect_session_events()?;
        self.queue.enqueue(events);

        Ok(())
    }
}
