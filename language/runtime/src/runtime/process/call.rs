#![allow(clippy::missing_const_for_thread_local)]

use std::any::Any;
use std::cell::{Cell, Ref, RefCell};
use std::ptr;
use std::time::Duration;

use crate::diagnostic::{DiagnosticStore, RuntimeError, RuntimeResult};
use crate::host::HostSession;
use crate::platform::{NativeArray, PlatformError, core as core_platform};
use crate::runtime::bindings::{
    BindingAffinity, BindingDescriptor, BindingEngine, BindingPolicy, BindingReplayPayload,
    RuntimeWorld,
};
use crate::runtime::policy::BindingDispatchDecision;
use crate::runtime::random::RandomStreamId;
use crate::runtime::scheduler::{
    EventLoop, EventLoopScope, MicrotaskId, TaskId, current_event_loop_scope,
};
use crate::runtime::trace::{EntropySubject, Trace};
use crate::runtime::world::World;
use crate::simulation::Simulation;

use super::{Agent, ExecutionContext, ExecutionContextId, binding_affinity_name};
use crate::runtime::{Hooks, NativeSlice, NativeStringRef, NativeStringSlice, PolicyCallId};
use destack_workspace::{RuntimeAccess, RuntimeDiagnosticLevel, TimeMode};

thread_local! {
    /// TLS slot for the current runtime execution context.
    static CURRENT_AGENT_CONTEXT: Cell<CurrentAgentContext> = const { Cell::new(CurrentAgentContext::empty()) };
    /// TLS slot for the current binding call context.
    static BINDING_CALL_CONTEXT: Cell<*const BindingCallContext> = const { Cell::new(ptr::null()) };
    /// TLS storage for native ABI references returned by bindings.
    static BINDING_CALL_ARENA: BindingCallArena = const { BindingCallArena::new() };
}

/// Current runtime execution context for VM callback bridging.
#[derive(Debug, Clone, Copy)]
struct CurrentAgentContext {
    /// Agent pointer for callback dispatch.
    agent: *const Agent,
    /// Event loop pointer for callback dispatch.
    event_loop: *const EventLoop,
    /// Host pointer for callback dispatch.
    host: *const HostSession,
    /// World pointer for replay, time, random, and policy.
    world: *const World,
    /// Execution context identifier for callback dispatch.
    execution_context_id: ExecutionContextId,
    /// Whether this execution scope runs on the process main context.
    is_process_main: bool,
}

impl CurrentAgentContext {
    /// Return one empty runtime execution context.
    const fn empty() -> Self {
        Self {
            agent: ptr::null(),
            event_loop: ptr::null(),
            host: ptr::null(),
            world: ptr::null(),
            execution_context_id: ExecutionContextId(0),
            is_process_main: false,
        }
    }

    /// Return whether this execution context is available.
    const fn is_empty(self) -> bool {
        self.agent.is_null()
            || self.event_loop.is_null()
            || self.host.is_null()
            || self.world.is_null()
    }
}

/// Guard that restores the previous current-agent execution context.
#[derive(Debug)]
pub(crate) struct CurrentAgentContextGuard {
    /// Previous current-agent execution context.
    previous: CurrentAgentContext,
}

impl Drop for CurrentAgentContextGuard {
    /// Restore the previous current-agent execution context.
    fn drop(&mut self) {
        CURRENT_AGENT_CONTEXT.with(|slot| slot.set(self.previous));
    }
}

/// Enter one current-agent execution context for VM callback dispatch.
pub(crate) fn enter_current_agent_context(
    agent: *const Agent,
    event_loop: *const EventLoop,
    host: *const HostSession,
    world: *const World,
    is_process_main: bool,
) -> CurrentAgentContextGuard {
    let event_loop = unsafe { &*event_loop };
    let execution_context_id = event_loop.execution_context_id();
    let next = CurrentAgentContext {
        agent,
        event_loop: event_loop as *const EventLoop,
        host,
        world,
        execution_context_id,
        is_process_main,
    };
    let previous = CURRENT_AGENT_CONTEXT.with(|slot| {
        let previous = slot.get();
        slot.set(next);
        previous
    });

    CurrentAgentContextGuard { previous }
}

/// Return the current-agent execution context when available.
fn current_agent_context() -> Option<CurrentAgentContext> {
    CURRENT_AGENT_CONTEXT.with(|slot| {
        let context = slot.get();
        if context.is_empty() {
            return None;
        }

        Some(context)
    })
}

/// TLS payload for native runtime calls.
#[derive(Debug, Clone)]
pub struct BindingCallContext {
    /// Agent state for platform bindings.
    agent: *const Agent,
    /// Event loop for task queues and timers.
    event_loop: *const EventLoop,
    /// Host state for platform callbacks.
    host: *const HostSession,
    /// Shared world for replay, time, random, and policy.
    world: *const World,
    /// Engine kind for this binding call.
    engine: BindingEngine,
    /// Event loop scope metadata for the current call.
    scope: EventLoopScope,
    /// Execution-affinity context for the current call.
    execution_context: ExecutionContext,
}

/// Scope guard that runs after-binding hooks when one binding call completes.
#[derive(Debug)]
pub struct BindingHookGuard<'call> {
    /// Binding call context for hook dispatch.
    context: &'call BindingCallContext,
    /// Binding descriptor for hook dispatch.
    spec: BindingDescriptor,
    /// Binding call identifier for before and after correlation.
    call_id: PolicyCallId,
}

impl Drop for BindingHookGuard<'_> {
    /// Run post-call hooks for this binding call scope.
    fn drop(&mut self) {
        self.context.on_after_binding(self.spec, self.call_id);
    }
}

impl BindingCallContext {
    /// Create a binding call context for TLS.
    pub fn new(agent: &Agent, event_loop: &EventLoop, host: &HostSession, world: &World) -> Self {
        let execution_context = event_loop.execution_context(host.is_process_main_context());

        Self {
            agent,
            event_loop,
            host,
            world,
            engine: BindingEngine::Native,
            scope: current_event_loop_scope(),
            execution_context,
        }
    }

    /// Create a binding call context from raw pointers.
    pub(crate) fn from_raw(
        agent: *const Agent,
        event_loop: *const EventLoop,
        host: *const HostSession,
        world: *const World,
        engine: BindingEngine,
    ) -> Self {
        let event_loop = unsafe { &*event_loop };
        let host = unsafe { &*host };
        let execution_context = event_loop.execution_context(host.is_process_main_context());

        Self {
            agent,
            event_loop: event_loop as *const EventLoop,
            host: host as *const HostSession,
            world,
            engine,
            scope: current_event_loop_scope(),
            execution_context,
        }
    }

    /// Create one VM binding call context from the current-agent execution scope.
    pub(crate) fn from_current_agent_for_vm() -> RuntimeResult<Self> {
        let context = current_agent_context()
            .ok_or_else(|| RuntimeError::BindingCallContextMissing.boxed())?;
        Ok(Self::from_raw(
            context.agent,
            context.event_loop,
            context.host,
            context.world,
            BindingEngine::Vm,
        )
        .with_execution_context(ExecutionContext::new(
            context.execution_context_id,
            context.is_process_main,
        )))
    }

    /// Override the execution context for one raw binding call context.
    fn with_execution_context(mut self, execution_context: ExecutionContext) -> Self {
        self.execution_context = execution_context;
        self
    }

    /// Borrow the agent state.
    #[inline]
    pub fn agent(&self) -> &Agent {
        // safety: pointer is owned by the runtime caller
        unsafe { &*self.agent }
    }

    /// Borrow the runtime diagnostics store.
    #[inline]
    pub fn diagnostics(&self) -> &DiagnosticStore {
        self.agent().diagnostics.as_ref()
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

    /// Borrow the event loop.
    #[inline]
    pub fn event_loop(&self) -> &EventLoop {
        // safety: pointer is owned by the runtime
        unsafe { &*self.event_loop }
    }

    /// Borrow immutable process arguments.
    #[inline]
    pub fn platform_args(&self) -> &[String] {
        self.agent().platform_args()
    }

    /// Borrow the trace state.
    #[inline]
    pub fn trace(&self) -> &Trace {
        self.world().trace()
    }

    /// Borrow the binding policy for this agent.
    #[inline]
    fn policy(&self) -> parking_lot::RwLockReadGuard<'_, BindingPolicy> {
        self.agent().bindings.policy().read()
    }

    /// Build one entropy replay subject for the current call and one binding.
    pub fn entropy_subject(&self, spec: BindingDescriptor) -> EntropySubject {
        EntropySubject {
            runtime_id: self.agent().runtime_id,
            agent_id: self.agent().id,
            binding_id: spec.id,
            engine: Some(self.engine()),
            task_id: self.task_id(),
            microtask_id: self.microtask_id(),
        }
    }

    /// Borrow the runtime hook state.
    #[inline]
    pub fn hooks(&self) -> &Hooks {
        self.agent().hooks.as_ref()
    }

    /// Borrow the runtime host state.
    #[inline]
    pub fn host(&self) -> &HostSession {
        // safety: pointer is owned by the runtime caller
        unsafe { &*self.host }
    }

    /// Borrow the shared runtime world.
    #[inline]
    pub fn world(&self) -> &World {
        // safety: pointer is owned by the runtime caller
        unsafe { &*self.world }
    }

    /// Borrow one read guard for the simulation state.
    #[inline]
    pub fn read_simulation(&self) -> Ref<'_, Simulation> {
        self.world().read_simulation()
    }

    /// Return the current event loop scope.
    pub const fn scope(&self) -> EventLoopScope {
        self.scope
    }

    /// Return the execution-affinity context for this call.
    pub const fn execution_context(&self) -> ExecutionContext {
        self.execution_context
    }

    /// Return the current execution context identifier for this call.
    pub const fn execution_context_id(&self) -> ExecutionContextId {
        self.execution_context.id
    }

    /// Service runtime-owned host ingress for the active runtime.
    pub(crate) fn service_runtime_ingress(&self) -> RuntimeResult<()> {
        self.host().service_ingress()
    }

    /// Wait for one binding result while runtime-owned host ingress makes progress.
    pub fn wait_for_binding_result<T>(
        &self,
        operation: &'static str,
        timeout_message: &'static str,
        deadline_ns: u64,
        wait_slice_ns: u64,
        mut try_take: impl FnMut() -> RuntimeResult<Option<T>>,
        mut wait_once: impl FnMut(Duration),
    ) -> RuntimeResult<T> {
        // keep polling already-published state before servicing host ingress
        loop {
            if let Some(result) = try_take()? {
                return Ok(result);
            }

            // let the runtime and host own progress while the binding waits
            self.service_runtime_ingress()?;

            if let Some(result) = try_take()? {
                return Ok(result);
            }

            let now = core_platform::monotonic_now_ns();

            // stop once the timeout budget is exhausted
            if now >= deadline_ns {
                return Err(core_platform::io_would_block(operation, timeout_message));
            }

            // wait for the next backend publication within the remaining budget
            let remaining = deadline_ns.saturating_sub(now);
            let duration = Duration::from_nanos(remaining.min(wait_slice_ns));
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
        // resolve runtime and agent scoped stream selection policy
        let agent = self.agent();
        let is_per_runnable = agent.options.random.per_runnable;
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
        self.world().random().scoped_stream_id(
            agent.runtime_id.0,
            agent.id.0,
            task_id,
            microtask_id,
        )
    }

    /// Return true when the world clock runs in virtual mode.
    #[inline]
    pub fn is_virtual_clock(&self) -> bool {
        self.world().time_mode() == TimeMode::Virtual
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

    /// Notify policy hooks about one clock-read operation.
    #[inline]
    pub fn on_time_read(&self) {
        self.hooks().on_time_read(self.world(), Some(self.engine()));
    }

    /// Notify policy hooks about one random-read operation.
    #[inline]
    pub fn on_random_read(&self) {
        self.hooks()
            .on_random_read(self.world(), Some(self.engine()));
    }

    /// Sleep one runtime-backed duration.
    #[inline]
    pub fn sleep_nanos(&self, duration: u64) {
        self.world().clock().host_sleep_nanos(duration);
    }

    /// Sleep until one runtime-backed wall deadline.
    #[inline]
    pub fn sleep_until_wall_nanos(&self, deadline: u64) {
        self.world().clock().host_sleep_until_nanos(deadline);
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

    /// Return the engine kind for this call context.
    pub const fn engine(&self) -> BindingEngine {
        self.engine
    }

    /// Clear call-local storage for native bindings.
    pub fn clear_values(&self) {
        BINDING_CALL_ARENA.with(|arena| arena.clear());
    }

    /// Store a string for the duration of the current call.
    pub fn store_string(&self, value: &str) -> NativeStringRef {
        BINDING_CALL_ARENA.with(|arena| arena.store_string(value))
    }

    /// Store an optional string for the duration of the current call.
    pub fn store_string_option(&self, value: Option<&String>) -> NativeStringRef {
        BINDING_CALL_ARENA.with(|arena| arena.store_string_option(value))
    }

    /// Store a slice for the duration of the current call.
    pub fn store_slice<T: 'static>(&self, values: Vec<T>) -> NativeSlice<T> {
        BINDING_CALL_ARENA.with(|arena| arena.store_slice(values))
    }

    /// Store an array for the duration of the current call.
    pub fn store_array<T: 'static>(&self, values: Vec<T>) -> NativeArray<T> {
        BINDING_CALL_ARENA.with(|arena| arena.store_array(values))
    }

    /// Store a string slice for the duration of the current call.
    pub fn store_string_slice(&self, values: Vec<NativeStringRef>) -> NativeStringSlice {
        BINDING_CALL_ARENA.with(|arena| arena.store_string_slice(values))
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
        if execution_context_satisfies(
            self.execution_context(),
            self.event_loop().execution_context_id(),
            spec.affinity(),
            None,
        ) {
            return Ok(());
        }

        Err(self.affinity_violation_error(spec).boxed())
    }

    /// Ensure world access routing allows this binding call.
    fn ensure_binding_access_allowed(
        &self,
        spec: BindingDescriptor,
        decision: &BindingDispatchDecision,
    ) -> RuntimeResult<()> {
        if decision.access == RuntimeAccess::Deny {
            return Err(self.policy_violation_error(spec).boxed());
        }

        Ok(())
    }

    /// Run pre-call policy checks and return one dispatch decision snapshot.
    #[inline]
    fn preflight_binding_call(
        &self,
        spec: BindingDescriptor,
    ) -> RuntimeResult<BindingDispatchDecision> {
        // reject execution-affinity mismatches before policy and hooks
        self.ensure_binding_affinity_allowed(spec)?;

        // run policy checks before evaluating hooks
        let policy = self.policy();
        policy.ensure_allowed_for_engine(spec, Some(self.engine))?;
        let decision = self.world().resolve_binding_dispatch(
            self.hooks().execution_mode(),
            self.agent().runtime_id,
            self.agent().id,
            spec,
            Some(self.engine),
            policy.default_access(),
            policy.default_world(),
            policy.default_replay_payload(),
        )?;
        self.ensure_binding_access_allowed(spec, &decision)?;

        // service runtime-owned host ingress before host bindings execute
        if decision.world == RuntimeWorld::Host {
            self.service_runtime_ingress()?;
        }

        Ok(decision)
    }

    /// Run pre-call binding policy and return one post-call hook guard.
    #[inline]
    pub fn on_before_binding(
        &self,
        spec: BindingDescriptor,
    ) -> RuntimeResult<BindingHookGuard<'_>> {
        self.preflight_binding_call(spec)?;
        let call_id = self
            .hooks()
            .on_before_binding(self.world(), spec, Some(self.engine))?;

        Ok(BindingHookGuard {
            context: self,
            spec,
            call_id,
        })
    }

    /// Run pre-call policy, resolve world, and return one post-call hook guard.
    #[inline]
    pub fn on_before_binding_resolve_world(
        &self,
        spec: BindingDescriptor,
    ) -> RuntimeResult<(RuntimeWorld, BindingHookGuard<'_>)> {
        let decision = self.preflight_binding_call(spec)?;
        let world = decision.world;

        // reject host dispatch when the binding is unavailable on this host
        if world == RuntimeWorld::Host && !spec.supports_current_host() {
            return Err(RuntimeError::from(PlatformError::not_supported(spec.name)).boxed());
        }

        let call_id = self
            .hooks()
            .on_before_binding(self.world(), spec, Some(self.engine))?;

        let hook_guard = BindingHookGuard {
            context: self,
            spec,
            call_id,
        };
        Ok((world, hook_guard))
    }

    /// Run post-call hooks for one binding descriptor.
    #[inline]
    fn on_after_binding(&self, spec: BindingDescriptor, call_id: PolicyCallId) {
        self.hooks()
            .on_after_binding(self.world(), spec, Some(self.engine), call_id);
    }

    /// Resolve the binding world for this call context.
    #[inline]
    pub fn resolve_world(&self, spec: BindingDescriptor) -> RuntimeResult<RuntimeWorld> {
        let policy = self.policy();
        let decision = self.world().resolve_binding_dispatch(
            self.hooks().execution_mode(),
            self.agent().runtime_id,
            self.agent().id,
            spec,
            Some(self.engine),
            policy.default_access(),
            policy.default_world(),
            policy.default_replay_payload(),
        )?;

        Ok(decision.world)
    }

    /// Resolve the replay payload policy for this call context.
    #[inline]
    pub fn replay_payload_for(
        &self,
        spec: BindingDescriptor,
    ) -> RuntimeResult<BindingReplayPayload> {
        let policy = self.policy();
        let decision = self.world().resolve_binding_dispatch(
            self.hooks().execution_mode(),
            self.agent().runtime_id,
            self.agent().id,
            spec,
            Some(self.engine),
            policy.default_access(),
            policy.default_world(),
            policy.default_replay_payload(),
        )?;

        let requested = decision.replay_payload;
        self.trace().payload_policy_for_requested(spec, requested)
    }
}

/// Return whether one execution context satisfies one binding affinity requirement.
pub(crate) const fn execution_context_satisfies(
    execution_context: ExecutionContext,
    event_loop_context_id: ExecutionContextId,
    affinity: BindingAffinity,
    owner_execution_context_id: Option<ExecutionContextId>,
) -> bool {
    match affinity {
        BindingAffinity::Any => true,
        BindingAffinity::EventLoop => execution_context.id.0 == event_loop_context_id.0,
        BindingAffinity::Owner => match owner_execution_context_id {
            Some(owner_execution_context_id) => {
                execution_context.id.0 == owner_execution_context_id.0
            }
            None => false,
        },
        BindingAffinity::ProcessMain => execution_context.is_process_main,
    }
}

/// Guard that restores the previous TLS binding call context.
#[derive(Debug)]
pub struct BindingCallGuard {
    /// Previous TLS context pointer.
    previous: *const BindingCallContext,
}

impl Drop for BindingCallGuard {
    /// Restore the previous binding call context.
    fn drop(&mut self) {
        BINDING_CALL_CONTEXT.with(|slot| slot.set(self.previous));
    }
}

/// Enter a binding call context for native bindings.
#[inline]
pub fn enter_binding_call_context(context: &BindingCallContext) -> BindingCallGuard {
    // swap in the new TLS context and capture the previous one
    let previous = BINDING_CALL_CONTEXT.with(|slot| {
        let previous = slot.get();
        slot.set(context as *const BindingCallContext);
        previous
    });

    BindingCallGuard { previous }
}

/// Access the current binding call context for native bindings.
#[inline]
pub fn with_binding_call_context<T>(
    f: impl FnOnce(&BindingCallContext) -> RuntimeResult<T>,
) -> RuntimeResult<T> {
    // read the TLS context pointer
    let context = BINDING_CALL_CONTEXT.with(|slot| slot.get());
    if context.is_null() {
        return Err(RuntimeError::BindingCallContextMissing.boxed());
    }

    // safety: pointer is set by enter_binding_call_context
    let context = unsafe { &*context };

    // clear call-local value storage
    context.clear_values();
    f(context)
}

/// Per-call storage for native ABI references returned by bindings.
///
/// Stored pointers are valid until the next runtime call on the same native thread.
#[derive(Debug, Default)]
pub struct BindingCallArena {
    /// Owned strings backing native string references.
    strings: RefCell<Vec<Box<str>>>,
    /// Owned slices backing native slice references.
    values: RefCell<Vec<Box<dyn Any>>>,
}

impl BindingCallArena {
    /// Create an empty call arena.
    pub const fn new() -> Self {
        Self {
            strings: RefCell::new(Vec::new()),
            values: RefCell::new(Vec::new()),
        }
    }

    /// Clear all stored references.
    pub fn clear(&self) {
        self.strings.borrow_mut().clear();
        self.values.borrow_mut().clear();
    }

    /// Store a string and return a native string reference.
    pub fn store_string(&self, value: &str) -> NativeStringRef {
        let mut strings = self.strings.borrow_mut();
        strings.push(value.to_owned().into_boxed_str());

        let stored = strings.last().expect("stored string must be available");
        NativeStringRef::from(stored.as_ref())
    }

    /// Store an optional string and return a native string reference.
    pub fn store_string_option(&self, value: Option<&String>) -> NativeStringRef {
        match value {
            Some(value) => self.store_string(value),
            None => NativeStringRef {
                data: ptr::null(),
                len: 0,
            },
        }
    }

    /// Store a slice and return a native slice reference.
    pub fn store_slice<T: 'static>(&self, values: Vec<T>) -> NativeSlice<T> {
        let mut boxed = values.into_boxed_slice();
        let data = boxed.as_mut_ptr();
        let len = boxed.len() as u32;
        self.values.borrow_mut().push(Box::new(boxed));

        NativeSlice { data, len }
    }

    /// Store a slice and return a native array reference.
    pub fn store_array<T: 'static>(&self, values: Vec<T>) -> NativeArray<T> {
        let mut boxed = values.into_boxed_slice();
        let data = boxed.as_mut_ptr();
        let len = boxed.len() as u32;
        self.values.borrow_mut().push(Box::new(boxed));

        NativeArray {
            data,
            len,
            capacity: len,
        }
    }

    /// Store a string slice and return a native string slice.
    pub fn store_string_slice(&self, values: Vec<NativeStringRef>) -> NativeStringSlice {
        let mut boxed = values.into_boxed_slice();
        let data = boxed.as_mut_ptr() as *const NativeStringRef;
        let len = boxed.len() as u32;
        self.values.borrow_mut().push(Box::new(boxed));

        NativeStringSlice { data, len }
    }
}
