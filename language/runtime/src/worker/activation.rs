use std::time::Duration;

use destack_artifact::ConditionSet;
use destack_program as program;
use destack_repository::{Environment, RuntimeDiagnosticLevel};

use crate::binding::{Binding, BindingAccess, BindingTable, ReplayPayload};
use crate::diagnostic::{DiagnosticStore, RuntimeError, RuntimeResult};
use crate::host::{
    Host, HostError, HostQueue, family_name, host_name, monotonic_now_ns, platform_name,
};
use crate::scheduler::{EventLoop, RunnableId};
use crate::world::random::RandomStreamId;
use crate::world::time::ClockSource;
use crate::world::trace::{EntropySubject, TraceLog};
use crate::world::{Decision, RuntimeId, WorldState};

use super::{Handshake, Request, RunnableScope, WorkerId};

/// Runtime state exposed during one worker activation.
#[derive(Debug)]
pub struct Activation<'a> {
    /// Runtime owner identifier in world topology.
    pub(crate) runtime_id: RuntimeId,
    /// Worker identifier in world topology.
    pub(crate) worker_id: WorkerId,
    /// Immutable ambient environment for host bindings.
    environment: &'a Environment,
    /// The active runtime conditions.
    conditions: &'a ConditionSet,
    /// Durable program binding declarations.
    program: &'a program::Program,
    /// Runtime diagnostics storage.
    diagnostics: &'a DiagnosticStore,
    /// Worker-local binding access policy.
    binding_access: &'a BindingAccess,
    /// Runtime-shared binding implementations.
    binding_table: &'a BindingTable,
    /// Host integration for callbacks.
    host: &'a dyn Host,
    /// Host event queue for callbacks.
    host_queue: &'a HostQueue,
    /// Shared world for replay, time, random, and policy.
    world: &'a mut WorldState,
    /// Currently running task or microtask.
    scope: RunnableScope,
    /// Worker event loop receiving language waiter operations.
    event_loop: &'a mut EventLoop,
    /// Process-local worker execution handshake.
    handshake: &'a Handshake,
    /// Whether this call is running on the process main thread.
    is_process_main: bool,
}

impl program::Runtime for Activation<'_> {
    type Error = Box<RuntimeError>;

    /// Return whether execution must yield at the current runtime poll.
    fn is_poll_requested(&self) -> bool {
        self.handshake.is_pending()
    }

    /// Retain execution so the worker can service pending runtime work.
    fn poll(
        &mut self,
        _memory: program::Memory<'_>,
        _roots: &mut dyn program::RootSource<Error = Self::Error>,
    ) -> RuntimeResult<program::Poll> {
        if !self.handshake.is_pending() {
            return Ok(program::Poll::Continue);
        }

        Ok(program::Poll::Pause)
    }

    /// Call one linked runtime binding.
    fn call_binding(
        &mut self,
        memory: program::Memory<'_>,
        binding: &program::Binding,
        arguments: &[program::Word],
        result: &mut [program::Word],
    ) -> RuntimeResult<()> {
        let table = self.binding_table;

        table.call(binding, self, memory, arguments, result)
    }

    /// Attempt to queue one suspended language waiter.
    fn queue_waiter(
        &mut self,
        waiter: program::Waiter,
        value: program::Value,
    ) -> RuntimeResult<bool> {
        self.event_loop
            .queue_waiter(waiter, value)
            .map_err(Into::into)
    }

    /// Attempt to cancel one suspended language waiter.
    fn cancel_waiter(&mut self, waiter: program::Waiter) -> RuntimeResult<bool> {
        self.event_loop.cancel_waiter(waiter).map_err(Into::into)
    }

    /// Create one already completed task.
    fn resolve_task(&mut self, value: program::Value) -> program::Task {
        self.event_loop.resolve_task(value)
    }

    /// Start one running task.
    fn start_task(&mut self) -> program::Task {
        self.event_loop.start_task()
    }

    /// Suspend one running task or return its continuation unchanged.
    fn suspend_task(
        &mut self,
        task: program::Task,
        continuation: program::Continuation,
    ) -> Result<program::Waiter, (Box<RuntimeError>, program::Continuation)> {
        self.event_loop
            .suspend_task(task, continuation)
            .map_err(|(error, continuation)| (error.into(), continuation))
    }

    /// Park one waiter until a task completes or is cancelled.
    fn park_task(&mut self, task: program::Task, waiter: program::Waiter) -> RuntimeResult<()> {
        self.event_loop.park_task(task, waiter).map_err(Into::into)
    }

    /// Request cooperative cancellation of one task.
    fn cancel_task(&mut self, task: program::Task) -> RuntimeResult<()> {
        self.event_loop.cancel_task(task).map_err(Into::into)
    }

    /// Return whether cooperative cancellation was requested for one running task.
    fn is_task_cancelled(&mut self, task: program::Task) -> RuntimeResult<bool> {
        self.event_loop.is_task_cancelled(task).map_err(Into::into)
    }

    /// Detach one task result.
    fn detach_task(&mut self, task: program::Task) -> RuntimeResult<()> {
        self.event_loop.detach_task(task).map_err(Into::into)
    }

    /// Finish one running task.
    fn finish_task(
        &mut self,
        task: program::Task,
        outcome: program::TaskOutcome,
    ) -> RuntimeResult<()> {
        self.event_loop
            .finish_task(task, outcome)
            .map_err(Into::into)
    }
}

#[allow(clippy::too_many_arguments)]
impl<'a> Activation<'a> {
    /// Create one worker activation.
    pub(super) fn new(
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        environment: &'a Environment,
        conditions: &'a ConditionSet,
        program: &'a program::Program,
        diagnostics: &'a DiagnosticStore,
        binding_access: &'a BindingAccess,
        binding_table: &'a BindingTable,
        host: &'a dyn Host,
        host_queue: &'a HostQueue,
        world: &'a mut WorldState,
        scope: RunnableScope,
        event_loop: &'a mut EventLoop,
        handshake: &'a Handshake,
    ) -> Self {
        Self {
            runtime_id,
            worker_id,
            environment,
            conditions,
            program,
            diagnostics,
            binding_access,
            binding_table,
            host,
            host_queue,
            world,
            scope,
            event_loop,
            handshake,
            is_process_main: host.is_process_main_context(),
        }
    }

    /// Return the process-local request word address used by native code.
    pub(crate) fn poll_address(&self) -> *const u32 {
        self.handshake.address()
    }

    /// Request interpreter continuation at the next runtime boundary.
    pub(crate) fn deoptimize(&self) {
        self.handshake.request(Request::Deoptimize);
    }

    /// Borrow the runtime diagnostics store.
    #[inline]
    pub fn diagnostics(&self) -> &DiagnosticStore {
        self.diagnostics
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
    pub fn trace(&self) -> &TraceLog {
        &self.world.trace
    }

    /// Build one entropy replay subject for the current call and one binding.
    pub fn entropy_subject(&self, binding: program::BindingId) -> EntropySubject {
        EntropySubject {
            runtime_id: self.runtime_id,
            worker_id: self.worker_id,
            binding_id: binding,
            scope: self.scope(),
        }
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
        self.host_queue.advance(self.host)
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

            let now = monotonic_now_ns();

            // stop once the timeout budget is exhausted
            if now >= deadline_ns {
                return Err(
                    RuntimeError::from(HostError::would_block(operation, timeout_message)).boxed(),
                );
            }

            // wait until the next machine publication or the overall deadline
            let remaining = deadline_ns - now;
            let duration = Duration::from_nanos(remaining);
            wait_once(duration);
        }
    }

    /// Return the current task identifier.
    pub const fn task_id(&self) -> Option<RunnableId> {
        self.scope.task_id()
    }

    /// Return the current microtask identifier.
    pub const fn microtask_id(&self) -> Option<RunnableId> {
        self.scope.microtask_id()
    }

    /// Return the current random stream identifier.
    pub fn random_stream_id(&mut self) -> RandomStreamId {
        // resolve one stable worker scoped stream id
        self.world
            .random
            .worker_stream_id(self.runtime_id.0, self.worker_id.0)
    }

    /// Return true when the world clock is runtime-owned.
    #[inline]
    pub fn is_runtime_clock(&self) -> bool {
        self.world.clock.source() == ClockSource::Runtime
    }

    /// Return one runtime-backed wall clock sample.
    #[inline]
    pub fn wall_nanos(&self) -> u64 {
        self.world.wall_nanos()
    }

    /// Return one runtime-backed monotonic clock sample.
    #[inline]
    pub fn mono_nanos(&self) -> u64 {
        self.world.mono_nanos()
    }

    /// Sleep one runtime-backed duration.
    #[inline]
    pub fn sleep_nanos(&self, duration: u64) {
        self.host.sleep_nanos(duration);
    }

    /// Sleep until one runtime-backed wall deadline.
    #[inline]
    pub fn sleep_until_wall_nanos(&self, deadline: u64) {
        self.host.sleep_until_wall_nanos(deadline);
    }

    /// Sleep until one runtime-backed monotonic deadline.
    #[inline]
    pub fn sleep_until_mono_nanos(&self, deadline: u64) {
        let now = self.mono_nanos();
        if deadline <= now {
            return;
        }

        let delta = deadline - now;
        self.sleep_nanos(delta);
    }

    /// Run pre-call binding policy.
    #[inline]
    pub fn on_before_binding(&mut self, binding: &program::Binding) -> RuntimeResult<()> {
        let name = self.binding_name(binding)?;

        // execution affinity
        if !binding.affinity.allows(self.is_process_main()) {
            let affinity = binding.affinity.name().to_string();

            return Err(RuntimeError::affinity_violation(name.to_string(), affinity).boxed());
        }

        // execution mode
        self.binding_access.ensure_allowed(binding, name)?;

        // world policy
        if matches!(self.decide_binding(binding)?, Decision::Deny) {
            return Err(RuntimeError::policy_violation(name.to_string()).boxed());
        }

        // reject unavailable host binding before entering the call
        if !self.supports_current_target(binding) {
            return Err(RuntimeError::from(HostError::not_supported(name)).boxed());
        }

        // service runtime-owned host ingress before host binding execution
        self.advance_wait_progress()?;

        Ok(())
    }

    /// Resolve the replay payload policy for this call context.
    #[inline]
    pub fn replay_payload_for(&self, binding: Binding, name: &str) -> RuntimeResult<ReplayPayload> {
        let requested = self.binding_access.default_replay_payload();
        self.trace()
            .payload_policy_for_requested(binding, name, requested)
    }

    /// Resolve one binding access decision for this call context.
    #[inline]
    fn decide_binding(&self, binding: &program::Binding) -> RuntimeResult<Decision> {
        self.world.decide_binding(
            self.conditions,
            self.runtime_id,
            self.worker_id,
            self.program,
            binding,
        )
    }

    /// Return one Program binding name.
    pub fn binding_name(&self, binding: &program::Binding) -> RuntimeResult<&str> {
        self.program.string(binding.name).ok_or_else(|| {
            RuntimeError::Internal {
                message: format!("binding {:?} references a missing name", binding.id),
            }
            .boxed()
        })
    }

    /// Return whether one binding supports the current runtime target.
    fn supports_current_target(&self, binding: &program::Binding) -> bool {
        let platforms = self.program.binding_platforms(binding);
        let families = self.program.binding_families(binding);
        let hosts = self.program.binding_hosts(binding);
        if platforms.is_empty() && families.is_empty() && hosts.is_empty() {
            return true;
        }

        let platform = platform_name();
        let family = family_name();
        let host = host_name();
        platforms
            .iter()
            .any(|id| self.program.string(*id) == Some(platform))
            || families
                .iter()
                .any(|id| self.program.string(*id) == Some(family))
            || hosts
                .iter()
                .any(|id| self.program.string(*id) == Some(host))
    }
}
