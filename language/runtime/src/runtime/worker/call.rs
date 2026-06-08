use std::sync::Arc;
use std::time::Duration;

use crate::diagnostic::{DiagnosticStore, RuntimeError, RuntimeResult};
use crate::host::binding::{
    BindingAccess, BindingAffinity, BindingDescriptor, BindingRegistry, BindingReplayPayload,
    RuntimeAccess,
};
use crate::host::core::{Host, HostQueue, advance_host_events};
use crate::host::{HostError, core as host_core};
use crate::runtime::random::RandomStreamId;
use crate::runtime::scheduler::{MicrotaskId, TaskId};
use crate::runtime::time::ClockSource;
use crate::world::trace::{EntropySubject, Trace};
use crate::world::{RuntimeId, WorldState};

use super::{RunnableScope, WorkerId, binding_affinity_name};
use destack_repository::{Environment, RuntimeDiagnosticLevel, RuntimeOptions};

/// TLS payload for native runtime calls.
#[derive(Debug)]
pub struct BindingCall<'host> {
    /// Runtime owner identifier in world topology.
    pub(crate) runtime_id: RuntimeId,
    /// Worker identifier in world topology.
    pub(crate) worker_id: WorkerId,
    /// Immutable ambient environment for host bindings.
    pub(crate) environment: Arc<Environment>,
    /// Immutable runtime options.
    pub(crate) options: Arc<RuntimeOptions>,
    /// Runtime diagnostics storage.
    pub(crate) diagnostics: Arc<DiagnosticStore>,
    /// External binding registry and policy enforcement.
    pub(crate) bindings: *const BindingRegistry,
    /// Host integration for callbacks.
    pub(crate) host: &'host dyn Host,
    /// Host event queue for callbacks.
    pub(crate) host_queue: &'host HostQueue,
    /// Shared world for replay, time, random, and policy.
    pub(crate) world: *mut WorldState,
    /// Currently running task or microtask.
    pub(crate) scope: RunnableScope,
    /// Whether this call is running on the process main thread.
    pub(crate) is_process_main: bool,
}

#[allow(clippy::mut_from_ref)]
impl BindingCall<'_> {
    /// Borrow the binding registry.
    #[inline]
    fn bindings(&self) -> &BindingRegistry {
        // SAFETY: the pointer targets a worker field that is not mutably borrowed during calls
        unsafe { &*self.bindings }
    }

    /// Borrow the runtime diagnostics store.
    #[inline]
    pub fn diagnostics(&self) -> &DiagnosticStore {
        self.diagnostics.as_ref()
    }

    /// Record one runtime diagnostic event.
    pub fn record_diagnostic(
        &self,
        level: RuntimeDiagnosticLevel,
        module: &'static str,
        operation: &'static str,
        message: impl Into<String>,
        os_code: Option<u32>,
    ) {
        self.diagnostics()
            .record(level, module, operation, message.into(), os_code);
    }

    /// Record one runtime warning diagnostic event.
    pub fn warn(
        &self,
        module: &'static str,
        operation: &'static str,
        message: impl Into<String>,
        os_code: Option<u32>,
    ) {
        self.record_diagnostic(
            RuntimeDiagnosticLevel::Warn,
            module,
            operation,
            message,
            os_code,
        );
    }

    /// Borrow immutable launch arguments.
    #[inline]
    pub fn arguments(&self) -> &[String] {
        self.environment.args.as_slice()
    }

    /// Borrow the trace state.
    #[inline]
    pub fn trace(&self) -> &Trace {
        &self.world().trace
    }

    /// Borrow the binding access for this worker.
    #[inline]
    fn access(&self) -> parking_lot::RwLockReadGuard<'_, BindingAccess> {
        self.bindings().access().read()
    }

    /// Build one entropy replay subject for the current call and one binding.
    pub fn entropy_subject(&self, spec: BindingDescriptor) -> EntropySubject {
        EntropySubject {
            runtime_id: self.runtime_id,
            worker_id: self.worker_id,
            binding_id: spec.id,
            task_id: self.task_id(),
            microtask_id: self.microtask_id(),
        }
    }

    /// Borrow the host integration.
    #[inline]
    pub(crate) fn host(&self) -> &dyn Host {
        self.host
    }

    /// Borrow the host event queue.
    #[inline]
    pub(crate) fn host_queue(&self) -> &HostQueue {
        self.host_queue
    }

    /// Borrow the shared runtime world.
    #[inline]
    pub(crate) fn world(&self) -> &mut WorldState {
        // SAFETY: the worker tick owns exclusive world access while this context is active
        unsafe { &mut *self.world }
    }

    /// Return the currently running task or microtask.
    pub const fn scope(&self) -> RunnableScope {
        self.scope
    }

    /// Return whether this call is running on the process main thread.
    pub const fn is_process_main(&self) -> bool {
        self.is_process_main
    }

    /// Advance host and runtime wait progress for one blocked binding path.
    pub(crate) fn advance_wait_progress(&self) -> RuntimeResult<()> {
        advance_host_events(self.host(), self.host_queue())
    }

    /// Wait for one binding result while runtime-owned host ingress makes progress.
    pub fn wait_for_binding_result<T>(
        &self,
        operation: &'static str,
        timeout_message: &'static str,
        deadline_ns: u64,
        mut try_take: impl FnMut() -> RuntimeResult<Option<T>>,
        mut wait_once: impl FnMut(Duration),
    ) -> RuntimeResult<T> {
        // keep polling already-published state before servicing host ingress
        loop {
            if let Some(result) = try_take()? {
                return Ok(result);
            }

            // let the runtime and host own progress while the binding waits
            self.advance_wait_progress()?;

            if let Some(result) = try_take()? {
                return Ok(result);
            }

            let now = host_core::monotonic_now_ns();

            // stop once the timeout budget is exhausted
            if now >= deadline_ns {
                return Err(host_core::io_would_block(operation, timeout_message));
            }

            // wait until the next backend publication or the overall deadline
            let remaining = deadline_ns.saturating_sub(now);
            let duration = Duration::from_nanos(remaining);
            wait_once(duration);
        }
    }

    /// Return the current task identifier.
    pub const fn task_id(&self) -> Option<TaskId> {
        self.scope.task_id()
    }

    /// Return the current microtask identifier.
    pub const fn microtask_id(&self) -> Option<MicrotaskId> {
        self.scope.microtask_id()
    }

    /// Return the current random stream identifier.
    pub fn random_stream_id(&self) -> RandomStreamId {
        // resolve one stable worker scoped stream id
        self.world()
            .random
            .worker_stream_id(self.runtime_id.0, self.worker_id.0)
    }

    /// Return true when the world clock is runtime-owned.
    #[inline]
    pub fn is_runtime_clock(&self) -> bool {
        self.world().clock.source() == ClockSource::Runtime
    }

    /// Return one runtime-backed wall clock sample.
    #[inline]
    pub fn wall_nanos(&self) -> u64 {
        self.world().wall_nanos()
    }

    /// Return one runtime-backed monotonic clock sample.
    #[inline]
    pub fn mono_nanos(&self) -> u64 {
        self.world().mono_nanos()
    }

    /// Sleep one runtime-backed duration.
    #[inline]
    pub fn sleep_nanos(&self, duration: u64) {
        self.host().sleep_nanos(duration);
    }

    /// Sleep until one runtime-backed wall deadline.
    #[inline]
    pub fn sleep_until_wall_nanos(&self, deadline: u64) {
        self.host().sleep_until_wall_nanos(deadline);
    }

    /// Sleep until one runtime-backed monotonic deadline.
    #[inline]
    pub fn sleep_until_mono_nanos(&self, deadline: u64) {
        let now = self.mono_nanos();
        if deadline <= now {
            return;
        }

        let delta = deadline.saturating_sub(now);
        self.sleep_nanos(delta);
    }

    /// Return one policy-violation error for one binding descriptor.
    fn policy_violation_error(&self, spec: BindingDescriptor) -> RuntimeError {
        RuntimeError::policy_violation(spec.name.to_string())
    }

    /// Return one affinity-violation error for one binding descriptor.
    fn affinity_violation_error(&self, spec: BindingDescriptor) -> RuntimeError {
        RuntimeError::affinity_violation(
            spec.name.to_string(),
            binding_affinity_name(spec.affinity()).to_string(),
        )
    }

    /// Ensure the current binding call satisfies one affinity.
    fn ensure_binding_affinity_allowed(&self, spec: BindingDescriptor) -> RuntimeResult<()> {
        if binding_affinity_satisfied(self.is_process_main(), spec.affinity()) {
            return Ok(());
        }

        Err(self.affinity_violation_error(spec).boxed())
    }

    /// Ensure world access routing allows this binding call.
    fn ensure_binding_access_allowed(
        &self,
        spec: BindingDescriptor,
        access: RuntimeAccess,
    ) -> RuntimeResult<()> {
        if access == RuntimeAccess::Deny {
            return Err(self.policy_violation_error(spec).boxed());
        }

        Ok(())
    }

    /// Run pre-call binding policy.
    #[inline]
    pub fn on_before_binding(&self, spec: BindingDescriptor) -> RuntimeResult<()> {
        // reject execution-affinity mismatches before policy checks
        self.ensure_binding_affinity_allowed(spec)?;

        // access and policy
        let access = self.access();
        access.ensure_allowed(spec)?;
        let runtime_access = self.decide_binding(spec)?;
        self.ensure_binding_access_allowed(spec, runtime_access)?;

        // reject unavailable host bindings before entering the call
        if !spec.supports_current_target() {
            return Err(RuntimeError::from(HostError::not_supported(spec.name)).boxed());
        }

        // service runtime-owned host ingress before host bindings execute
        self.advance_wait_progress()?;

        Ok(())
    }

    /// Resolve the replay payload policy for this call context.
    #[inline]
    pub fn replay_payload_for(
        &self,
        spec: BindingDescriptor,
    ) -> RuntimeResult<BindingReplayPayload> {
        let access = self.access();
        let requested = access.default_replay_payload();
        self.trace().payload_policy_for_requested(spec, requested)
    }

    /// Resolve one binding access decision for this call context.
    #[inline]
    fn decide_binding(&self, spec: BindingDescriptor) -> RuntimeResult<RuntimeAccess> {
        self.world().decide_binding(
            &self.options.conditions,
            self.runtime_id,
            self.worker_id,
            spec,
        )
    }
}

/// Return whether one binding affinity requirement is satisfied.
pub(crate) const fn binding_affinity_satisfied(
    is_process_main: bool,
    affinity: BindingAffinity,
) -> bool {
    match affinity {
        BindingAffinity::None => true,
        BindingAffinity::Worker => true,
        BindingAffinity::Main => is_process_main,
    }
}
