use std::sync::Arc;
use std::time::Duration;

use crate::diagnostic::{DiagnosticStore, RuntimeError, RuntimeResult};
use crate::host::binding::{
    BindingAccess, BindingAffinity, BindingDescriptor, BindingRegistry, BindingReplayPayload,
    BindingRoute, RuntimeAccess,
};
use crate::host::core::{Host, HostQueue, advance_host_events};
use crate::host::{HostError, core as host_core};
use crate::runtime::random::RandomStreamId;
use crate::runtime::scheduler::{MicrotaskId, TaskId};
use crate::simulation::Simulation;
use crate::world::policy::BindingDecision;
use crate::world::scenario::{ScenarioCallId, ScenarioRunner};
use crate::world::trace::{EntropySubject, Trace};
use crate::world::{RuntimeId, WorldState};

use super::{ExecutionContext, RunnableScope, WorkerId, binding_affinity_name};
use destack_workspace::{ClockSource, Environment, RuntimeDiagnosticLevel, RuntimeOptions};

/// TLS payload for native runtime calls.
#[derive(Debug)]
pub struct BindingCallContext<'host> {
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
    /// Worker scenario runner.
    pub(crate) scenario: Arc<ScenarioRunner>,
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
    /// Execution-affinity context for the current call.
    pub(crate) execution_context: ExecutionContext,
}

/// Scope guard that records one after-binding event when one binding call completes.
#[derive(Debug)]
pub struct BindingCallGuard<'call> {
    /// Binding call context for event routing.
    context: &'call BindingCallContext<'call>,
    /// Binding descriptor for event routing.
    spec: BindingDescriptor,
    /// Binding call identifier for before and after correlation.
    call_id: ScenarioCallId,
}

impl Drop for BindingCallGuard<'_> {
    /// Record the after-binding event for this binding call scope.
    fn drop(&mut self) {
        self.context.on_after_binding(self.spec, self.call_id);
    }
}

#[allow(clippy::mut_from_ref)]
impl BindingCallContext<'_> {
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

    /// Borrow the scenario runner for this worker.
    #[inline]
    pub fn scenario(&self) -> &ScenarioRunner {
        self.scenario.as_ref()
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

    /// Borrow one read guard for the simulation state.
    #[inline]
    pub fn simulation(&self) -> &Simulation {
        &self.world().simulation
    }

    /// Return the currently running task or microtask.
    pub const fn scope(&self) -> RunnableScope {
        self.scope
    }

    /// Return the execution-affinity context for this call.
    pub const fn execution_context(&self) -> ExecutionContext {
        self.execution_context
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
        // resolve runtime and worker scoped stream selection policy
        let is_per_runnable = self.options.random_options().per_runnable;
        let task_id = if is_per_runnable {
            self.scope.task_id().map(TaskId::get)
        } else {
            None
        };
        let microtask_id = if is_per_runnable {
            self.scope.microtask_id().map(MicrotaskId::get)
        } else {
            None
        };

        // resolve one stable world scoped stream id
        self.world().random.scoped_stream_id(
            self.runtime_id.0,
            self.worker_id.0,
            task_id,
            microtask_id,
        )
    }

    /// Return true when the world clock runs in virtual mode.
    #[inline]
    pub fn is_virtual_clock(&self) -> bool {
        self.world().clock.source() == ClockSource::Virtual
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

    /// Notify scenario rules about one clock-read operation.
    #[inline]
    pub fn on_clock_read(&self) {
        if let Err(error) = self.scenario().on_clock_read(self.world()) {
            self.record_diagnostic(
                RuntimeDiagnosticLevel::Error,
                "runtime.scenario",
                "clockRead",
                error.message(),
                None,
            );
        }
    }

    /// Notify scenario rules about one random-read operation.
    #[inline]
    pub fn on_random_read(&self) {
        if let Err(error) = self.scenario().on_random_read(self.world()) {
            self.record_diagnostic(
                RuntimeDiagnosticLevel::Error,
                "runtime.scenario",
                "randomRead",
                error.message(),
                None,
            );
        }
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
        RuntimeError::PolicyViolation {
            name: spec.name.to_string(),
        }
    }

    /// Return one affinity-violation error for one binding descriptor.
    fn affinity_violation_error(&self, spec: BindingDescriptor) -> RuntimeError {
        RuntimeError::AffinityViolation {
            name: spec.name.to_string(),
            affinity: binding_affinity_name(spec.affinity()).to_string(),
        }
    }

    /// Ensure the current execution context satisfies one binding affinity.
    fn ensure_binding_affinity_allowed(&self, spec: BindingDescriptor) -> RuntimeResult<()> {
        if execution_context_satisfies(self.execution_context(), spec.affinity()) {
            return Ok(());
        }

        Err(self.affinity_violation_error(spec).boxed())
    }

    /// Ensure world access routing allows this binding call.
    fn ensure_binding_access_allowed(
        &self,
        spec: BindingDescriptor,
        decision: &BindingDecision,
    ) -> RuntimeResult<()> {
        if decision.access == RuntimeAccess::Deny {
            return Err(self.policy_violation_error(spec).boxed());
        }

        Ok(())
    }

    /// Run pre-call policy checks and return one binding decision.
    #[inline]
    fn preflight_binding_call(&self, spec: BindingDescriptor) -> RuntimeResult<BindingDecision> {
        // reject execution-affinity mismatches before policy and scenario events
        self.ensure_binding_affinity_allowed(spec)?;

        // access and policy
        let access = self.access();
        access.ensure_allowed(spec)?;
        let decision = self.decide_binding(spec)?;
        self.ensure_binding_access_allowed(spec, &decision)?;

        // service runtime-owned host ingress before host bindings execute
        if decision.route == BindingRoute::Host {
            self.advance_wait_progress()?;
        }

        Ok(decision)
    }

    /// Run pre-call binding policy and return one after-binding event guard.
    #[inline]
    pub fn on_before_binding(
        &self,
        spec: BindingDescriptor,
    ) -> RuntimeResult<BindingCallGuard<'_>> {
        self.preflight_binding_call(spec)?;
        let call_id = self.scenario().on_before_binding(self.world(), spec)?;

        Ok(BindingCallGuard {
            context: self,
            spec,
            call_id,
        })
    }

    /// Run pre-call policy, resolve world, and return one after-binding event guard.
    #[inline]
    pub fn on_before_binding_resolve_route(
        &self,
        spec: BindingDescriptor,
    ) -> RuntimeResult<(BindingRoute, BindingCallGuard<'_>)> {
        let decision = self.preflight_binding_call(spec)?;
        let route = decision.route;

        // reject unavailable host bindings before entering the call
        if route == BindingRoute::Host && !spec.supports_current_target() {
            return Err(RuntimeError::from(HostError::not_supported(spec.name)).boxed());
        }

        let call_id = self.scenario().on_before_binding(self.world(), spec)?;

        let call_guard = BindingCallGuard {
            context: self,
            spec,
            call_id,
        };
        Ok((route, call_guard))
    }

    /// Record one after-binding event.
    #[inline]
    fn on_after_binding(&self, spec: BindingDescriptor, call_id: ScenarioCallId) {
        if let Err(error) = self
            .scenario()
            .on_after_binding(self.world(), spec, call_id)
        {
            self.record_diagnostic(
                RuntimeDiagnosticLevel::Error,
                "runtime.scenario",
                "bindingAfter",
                error.message(),
                None,
            );
        }
    }

    /// Resolve the binding route for this call context.
    #[inline]
    pub fn resolve_route(&self, spec: BindingDescriptor) -> RuntimeResult<BindingRoute> {
        let decision = self.decide_binding(spec)?;

        Ok(decision.route)
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

    /// Resolve one binding policy decision for this call context.
    #[inline]
    fn decide_binding(&self, spec: BindingDescriptor) -> RuntimeResult<BindingDecision> {
        self.world().decide_binding(
            self.scenario().execution_mode(),
            &self.options.conditions,
            self.runtime_id,
            self.worker_id,
            spec,
        )
    }
}

/// Return whether one execution context satisfies one binding affinity requirement.
pub(crate) const fn execution_context_satisfies(
    execution_context: ExecutionContext,
    affinity: BindingAffinity,
) -> bool {
    match affinity {
        BindingAffinity::None => true,
        BindingAffinity::Worker => true,
        BindingAffinity::Main => execution_context.is_process_main,
    }
}
