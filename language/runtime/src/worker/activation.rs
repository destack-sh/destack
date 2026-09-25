use std::time::Duration;

use tspp_artifact::ConditionSet;
use tspp_program as program;
use tspp_repository::{Environment, RuntimeDiagnosticLevel};

use crate::binding::{Binding, BindingAccess, BindingTable, ReplayPayload};
use crate::diagnostic::{DiagnosticStore, RuntimeError, RuntimeResult};
use crate::host::{
    Host, HostError, HostQueue, family_name, host_name, monotonic_now_ns, platform_name,
};
use crate::runtime::RuntimeId;
use crate::scheduler::{EventLoop, Invocation, RunnableId};
use crate::world::random::RandomStreamId;
use crate::world::time::ClockSource;
use crate::world::trace::{EntropySubject, TraceLog};
use crate::world::{Decision, WorldState};

use super::{Handshake, RunnableScope, WorkerId};

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
    /// Fiber identity mounted by the current invocation.
    fiber_id: Option<program::FiberId>,
    /// Worker event loop receiving fiber scheduling operations.
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
    fn poll(&mut self, _memory: program::Memory<'_>) -> RuntimeResult<program::Poll> {
        if !self.handshake.is_pending() {
            return Ok(program::Poll::Continue);
        }

        Ok(program::Poll::Pause)
    }

    /// Return the Program event categories probed for this Worker.
    fn events(&self) -> program::EventSet {
        self.world
            .debugger
            .probe_events(self.runtime_id, self.worker_id)
    }

    /// Process one instrumentable Program execution event.
    fn observe(
        &mut self,
        fiber_id: Option<program::FiberId>,
        event: program::Event,
    ) -> RuntimeResult<()> {
        self.world
            .probe(self.runtime_id, self.worker_id, fiber_id, event)
    }

    /// Call one linked runtime binding.
    fn call_binding(
        &mut self,
        memory: program::Memory<'_>,
        context: program::Context,
        fiber_id: Option<program::FiberId>,
        binding: &program::Binding,
        arguments: &[program::Word],
        result: &mut [program::Word],
    ) -> RuntimeResult<()> {
        let table = self.binding_table;

        table.call(binding, self, memory, context, fiber_id, arguments, result)
    }

    /// Park one logical fiber unless a wake already settled.
    fn park(&mut self, fiber_id: program::FiberId) -> RuntimeResult<program::Park> {
        match self.event_loop.take_pending_wake(fiber_id)? {
            Some(value) => Ok(program::Park::Ready(value)),
            None => Ok(program::Park::Parked),
        }
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
        fiber_id: Option<program::FiberId>,
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
            fiber_id,
            event_loop,
            handshake,
            is_process_main: host.is_process_main_context(),
        }
    }

    /// Return the process-local request word address used by native code.
    pub(crate) fn poll_address(&self) -> *const u32 {
        self.handshake.address()
    }

    /// Borrow the runtime diagnostics store.
    #[inline]
    pub fn diagnostics(&self) -> &DiagnosticStore {
        self.diagnostics
    }

    /// Borrow the durable program binding declarations.
    #[inline]
    pub fn program(&self) -> &program::Program {
        self.program
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

    /// Borrow immutable process arguments.
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

    /// Return the fiber identity mounted by the current invocation.
    pub const fn fiber_id(&self) -> Option<program::FiberId> {
        self.fiber_id
    }

    /// Deliver one wake, buffering it until the target fiber parks.
    pub fn wake_fiber(
        &mut self,
        fiber_id: program::FiberId,
        value: program::Value,
    ) -> RuntimeResult<()> {
        self.event_loop.wake_fiber(fiber_id, value)
    }

    /// Queue one callback to run before the next task.
    pub fn queue_microtask(
        &mut self,
        function: program::FunctionId,
        environment: Option<program::Value>,
        arguments: Vec<program::Value>,
        context: program::Context,
    ) -> RunnableId {
        self.event_loop.enqueue_microtask(Invocation::Function {
            function,
            environment,
            arguments,
            context,
        })
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
