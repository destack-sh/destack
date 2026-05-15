use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::binding::BindingDescriptor;
use crate::runtime::WorkerId;
use crate::world::policy::Attempt;
use crate::world::{RuntimeId, WorldState};
use destack_source::matches as glob_matches;
use destack_workspace::{ConditionSet, ExecutionMode};

use super::{Fault, FaultRuleId, FaultTarget};

/// Hook for runtime action rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Hook {
    /// Trigger before invoking one binding implementation.
    BindingBefore,
    /// Trigger after invoking one binding implementation.
    BindingAfter,
    /// Trigger when one task is enqueued.
    SchedulerEnqueue,
    /// Trigger when one task is dequeued.
    SchedulerDequeue,
    /// Trigger when one timer fires.
    SchedulerTimerFire,
    /// Trigger when one ingress event is enqueued into the worker loop.
    IngressEnqueue,
    /// Trigger when time is read.
    TimeRead,
    /// Trigger when random data is read.
    RandomRead,
    /// Trigger when one resource is attached.
    ResourceAttach,
    /// Trigger when one resource is detached.
    ResourceDetach,
}

/// Stable identifier for one binding call scenario event pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScenarioCallId(pub u64);

/// Scenario event payload emitted by one runtime hook point.
#[derive(Debug, Clone, Copy)]
pub enum HookEvent {
    /// Event fired before invoking one binding.
    BindingBefore {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Binding call identifier for before and after correlation.
        call_id: ScenarioCallId,
        /// Binding metadata for this event.
        descriptor: BindingDescriptor,
        /// Monotonic timestamp for this event.
        time_ns: u64,
    },
    /// Event fired after invoking one binding.
    BindingAfter {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Binding call identifier for before and after correlation.
        call_id: ScenarioCallId,
        /// Binding metadata for this event.
        descriptor: BindingDescriptor,
        /// Monotonic timestamp for this event.
        time_ns: u64,
    },
    /// Event fired when one task is enqueued.
    SchedulerEnqueue {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Monotonic timestamp for this event.
        time_ns: u64,
    },
    /// Event fired when one task is dequeued.
    SchedulerDequeue {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Monotonic timestamp for this event.
        time_ns: u64,
    },
    /// Event fired when one timer fires.
    SchedulerTimerFire {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Monotonic timestamp for this event.
        time_ns: u64,
    },
    /// Event fired when one ingress event is enqueued.
    IngressEnqueue {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Monotonic timestamp for this event.
        time_ns: u64,
    },
    /// Event fired when time is read.
    TimeRead {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Monotonic timestamp for this event.
        time_ns: u64,
    },
    /// Event fired when random data is read.
    RandomRead {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Monotonic timestamp for this event.
        time_ns: u64,
    },
    /// Event fired when one resource is attached.
    ResourceAttach {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Monotonic timestamp for this event.
        time_ns: u64,
    },
    /// Event fired when one resource is detached.
    ResourceDetach {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Monotonic timestamp for this event.
        time_ns: u64,
    },
}

/// Durable hook state captured at one checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookSnapshot {
    /// The next scenario call identifier to allocate.
    pub next_call_id: u64,
    /// The number of scenario faults deferred to a later scenario executor.
    pub deferred_faults: u64,
}

/// One fault accepted by trigger evaluation for one scenario event.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct TriggeredFault {
    /// Stable identifier of the rule that fired.
    pub rule_id: FaultRuleId,
    /// Hook that produced this action.
    pub hook: Hook,
    /// Worker identifier for this action.
    pub worker_id: WorkerId,
    /// Binding call identifier when one call event fired this fault.
    pub call_id: Option<ScenarioCallId>,
    /// Fault emitted by this scenario rule.
    pub fault: Fault,
}

impl HookEvent {
    /// Return the hook kind for this event.
    pub(crate) const fn hook(&self) -> Hook {
        match self {
            Self::BindingBefore { .. } => Hook::BindingBefore,
            Self::BindingAfter { .. } => Hook::BindingAfter,
            Self::SchedulerEnqueue { .. } => Hook::SchedulerEnqueue,
            Self::SchedulerDequeue { .. } => Hook::SchedulerDequeue,
            Self::SchedulerTimerFire { .. } => Hook::SchedulerTimerFire,
            Self::IngressEnqueue { .. } => Hook::IngressEnqueue,
            Self::TimeRead { .. } => Hook::TimeRead,
            Self::RandomRead { .. } => Hook::RandomRead,
            Self::ResourceAttach { .. } => Hook::ResourceAttach,
            Self::ResourceDetach { .. } => Hook::ResourceDetach,
        }
    }

    /// Return the worker identifier for this event.
    pub(crate) const fn worker_id(&self) -> WorkerId {
        match self {
            Self::BindingBefore { worker_id, .. }
            | Self::BindingAfter { worker_id, .. }
            | Self::SchedulerEnqueue { worker_id, .. }
            | Self::SchedulerDequeue { worker_id, .. }
            | Self::SchedulerTimerFire { worker_id, .. }
            | Self::IngressEnqueue { worker_id, .. }
            | Self::TimeRead { worker_id, .. }
            | Self::RandomRead { worker_id, .. }
            | Self::ResourceAttach { worker_id, .. }
            | Self::ResourceDetach { worker_id, .. } => *worker_id,
        }
    }

    /// Return one binding descriptor when present.
    pub(crate) const fn binding_descriptor(&self) -> Option<BindingDescriptor> {
        match self {
            Self::BindingBefore { descriptor, .. } | Self::BindingAfter { descriptor, .. } => {
                Some(*descriptor)
            }
            _ => None,
        }
    }

    /// Return one call id when this event is one binding call event.
    pub(crate) const fn call_id(&self) -> Option<ScenarioCallId> {
        match self {
            Self::BindingBefore { call_id, .. } | Self::BindingAfter { call_id, .. } => {
                Some(*call_id)
            }
            _ => None,
        }
    }

    /// Return attempt facts for selector matching.
    pub(crate) const fn attempt(&self) -> Attempt {
        Attempt {
            binding: self.binding_descriptor(),
        }
    }

    /// Return whether this event increments call-scoped trigger counters.
    pub(crate) const fn counts_as_call_event(&self) -> bool {
        matches!(self, Self::BindingBefore { .. })
    }

    /// Return the monotonic timestamp for this event.
    pub(crate) const fn time_ns(&self) -> u64 {
        match self {
            Self::BindingBefore { time_ns, .. }
            | Self::BindingAfter { time_ns, .. }
            | Self::SchedulerEnqueue { time_ns, .. }
            | Self::SchedulerDequeue { time_ns, .. }
            | Self::SchedulerTimerFire { time_ns, .. }
            | Self::IngressEnqueue { time_ns, .. }
            | Self::TimeRead { time_ns, .. }
            | Self::RandomRead { time_ns, .. }
            | Self::ResourceAttach { time_ns, .. }
            | Self::ResourceDetach { time_ns, .. } => *time_ns,
        }
    }
}

/// Selector clauses for hook callback matching.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct HookSelector {
    /// Glob selector for one binding name.
    pub binding: Option<String>,
}

impl HookSelector {
    /// Match all events for the selected hook.
    pub fn any() -> Self {
        Self::default()
    }

    /// Match one binding glob for binding hook events.
    pub fn binding(pattern: impl Into<String>) -> Self {
        Self {
            binding: Some(pattern.into()),
        }
    }
}

/// Callback decision for one hook event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookDecision {
    /// Continue normal runtime processing.
    Allow,
    /// Deny processing for this event.
    Deny {
        /// Human-readable denial message.
        message: String,
    },
}

/// Stable identifier for one registered hook callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HookCallbackId(pub u64);

/// Hook callback function signature.
pub type HookCallback = Arc<dyn Fn(&HookEvent) -> HookDecision + Send + Sync + 'static>;

/// One callback registration in the hook registry.
struct HookRegistration {
    /// Stable callback identifier.
    pub id: HookCallbackId,
    /// Hook point this callback listens to.
    pub hook: Hook,
    /// Selector clauses for this callback.
    pub selector: HookSelector,
    /// Callback function for this registration.
    pub callback: HookCallback,
}

impl std::fmt::Debug for HookRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookRegistration")
            .field("id", &self.id)
            .field("hook", &self.hook)
            .field("selector", &self.selector)
            .finish()
    }
}

/// Registry for callback-style hook observers and interceptors.
#[derive(Default)]
pub(crate) struct HookRegistry {
    /// Next callback id sequence.
    next_callback_id: u64,
    /// Registered callbacks in registration order.
    callbacks: Vec<HookRegistration>,
}

impl std::fmt::Debug for HookRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookRegistry")
            .field("next_callback_id", &self.next_callback_id)
            .field("callback_count", &self.callbacks.len())
            .finish()
    }
}

impl HookRegistry {
    /// Register one callback and return its stable id.
    pub(crate) fn register(
        &mut self,
        hook: Hook,
        selector: HookSelector,
        callback: HookCallback,
    ) -> HookCallbackId {
        self.next_callback_id = self.next_callback_id.saturating_add(1);
        let callback_id = HookCallbackId(self.next_callback_id);
        self.callbacks.push(HookRegistration {
            id: callback_id,
            hook,
            selector,
            callback,
        });

        callback_id
    }

    /// Remove one callback by id and return whether one callback was removed.
    pub(crate) fn unregister(&mut self, callback_id: HookCallbackId) -> bool {
        let before_len = self.callbacks.len();
        self.callbacks.retain(|callback| callback.id != callback_id);
        self.callbacks.len() < before_len
    }

    /// Run matching callbacks in registration order.
    pub(crate) fn run_callbacks(&self, event: &HookEvent) -> HookDecision {
        for callback in &self.callbacks {
            if !self.callback_matches_event(callback, event) {
                continue;
            }

            let decision = (callback.callback)(event);
            if matches!(decision, HookDecision::Deny { .. }) {
                return decision;
            }
        }

        HookDecision::Allow
    }

    /// Return true when one callback registration matches one event.
    fn callback_matches_event(&self, callback: &HookRegistration, event: &HookEvent) -> bool {
        if callback.hook != event.hook() {
            return false;
        }

        let Some(binding_pattern) = &callback.selector.binding else {
            return true;
        };
        let Some(descriptor) = event.binding_descriptor() else {
            return false;
        };

        glob_match(binding_pattern, descriptor.name)
    }
}

/// Runtime hook callbacks and action state.
pub struct Hooks {
    /// Runtime identifier for selector matching.
    runtime_id: RuntimeId,
    /// Worker identifier for selector matching.
    worker_id: WorkerId,
    /// Execution mode used for rule matching.
    mode: ExecutionMode,
    /// Source graph conditions used for rule matching.
    conditions: ConditionSet,
    /// Callback-style hook registry.
    registry: RwLock<HookRegistry>,
    /// Next scenario call identifier sequence.
    next_call_id: AtomicU64,
    /// Total scenario faults deferred to a later scenario executor.
    deferred_faults: AtomicU64,
}

impl std::fmt::Debug for Hooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let callback_count = self.registry.read().callbacks.len();
        let deferred_faults = self.deferred_faults.load(Ordering::Relaxed);

        f.debug_struct("Hooks")
            .field("runtime_id", &self.runtime_id)
            .field("worker_id", &self.worker_id)
            .field("mode", &self.mode)
            .field("conditions", &self.conditions)
            .field("callback_count", &callback_count)
            .field("deferred_faults", &deferred_faults)
            .finish()
    }
}

impl Hooks {
    /// Create runtime hooks for one worker in one world.
    pub(crate) fn new(
        runtime_id: RuntimeId,
        worker_id: WorkerId,
        mode: ExecutionMode,
        conditions: ConditionSet,
    ) -> Self {
        Self {
            runtime_id,
            worker_id,
            mode,
            conditions,
            registry: RwLock::new(HookRegistry::default()),
            next_call_id: AtomicU64::new(1),
            deferred_faults: AtomicU64::new(0),
        }
    }

    /// Return execution mode for scenario matching.
    pub(crate) fn execution_mode(&self) -> ExecutionMode {
        self.mode
    }

    // capture barrier
    fn capture_barrier(&self) -> RuntimeResult<()> {
        // require no registered callbacks
        if !self.registry.read().callbacks.is_empty() {
            return Err(RuntimeError::CaptureBarrier {
                component: "runtime.hooks".to_string(),
                mode: "snapshot".to_string(),
                detail: "callbacks are still registered".to_string(),
            }
            .boxed());
        }

        // require no deferred scenario faults
        if self.deferred_faults.load(Ordering::Relaxed) != 0 {
            return Err(RuntimeError::CaptureBarrier {
                component: "runtime.hooks".to_string(),
                mode: "snapshot".to_string(),
                detail: "scenario faults are still pending".to_string(),
            }
            .boxed());
        }

        Ok(())
    }

    /// Capture one durable hook snapshot.
    pub(crate) fn snapshot(&self) -> RuntimeResult<HookSnapshot> {
        self.capture_barrier()?;

        Ok(HookSnapshot {
            next_call_id: self.next_call_id.load(Ordering::Relaxed),
            deferred_faults: self.deferred_faults.load(Ordering::Relaxed),
        })
    }

    /// Fork one quiescent hook state for one child worker.
    pub(crate) fn try_fork(&self) -> RuntimeResult<Option<Self>> {
        // require one quiescent hook state first
        let snapshot = match self.snapshot() {
            Ok(snapshot) => snapshot,
            Err(_) => return Ok(None),
        };

        // rebuild one fresh hook container with the same scalar state
        let forked = Self::new(
            self.runtime_id,
            self.worker_id,
            self.mode,
            self.conditions.clone(),
        );
        forked.restore_snapshot(&snapshot)?;

        Ok(Some(forked))
    }

    /// Restore one durable hook snapshot.
    pub(crate) fn restore_snapshot(&self, snapshot: &HookSnapshot) -> RuntimeResult<()> {
        self.capture_barrier()?;
        self.next_call_id
            .store(snapshot.next_call_id, Ordering::Relaxed);
        self.deferred_faults
            .store(snapshot.deferred_faults, Ordering::Relaxed);

        Ok(())
    }

    /// Register one callback for one hook point.
    pub fn on(&self, hook: Hook, selector: HookSelector, callback: HookCallback) -> HookCallbackId {
        let mut registry = self.registry.write();

        registry.register(hook, selector, callback)
    }

    /// Register one callback for one hook point with one closure.
    pub fn on_fn(
        &self,
        hook: Hook,
        selector: HookSelector,
        callback: impl Fn(&HookEvent) -> HookDecision + Send + Sync + 'static,
    ) -> HookCallbackId {
        self.on(hook, selector, Arc::new(callback))
    }

    /// Register one callback for binding-before hook events.
    pub fn on_before(
        &self,
        selector: HookSelector,
        callback: impl Fn(&HookEvent) -> HookDecision + Send + Sync + 'static,
    ) -> HookCallbackId {
        self.on_fn(Hook::BindingBefore, selector, callback)
    }

    /// Register one callback for binding-after hook events.
    pub fn on_after(
        &self,
        selector: HookSelector,
        callback: impl Fn(&HookEvent) -> HookDecision + Send + Sync + 'static,
    ) -> HookCallbackId {
        self.on_fn(Hook::BindingAfter, selector, callback)
    }

    /// Remove one callback by id.
    pub fn off(&self, callback_id: HookCallbackId) -> bool {
        let mut registry = self.registry.write();

        registry.unregister(callback_id)
    }

    /// Evaluate pre-call scenario hooks for one binding invocation.
    pub(crate) fn on_before_binding(
        &self,
        world: &mut WorldState,
        descriptor: BindingDescriptor,
    ) -> RuntimeResult<ScenarioCallId> {
        // allocate one call id for before and after correlation
        let call_id = ScenarioCallId(self.next_call_id.fetch_add(1, Ordering::Relaxed));

        let decision = self.on_scenario_event(
            world,
            HookEvent::BindingBefore {
                worker_id: self.worker_id,
                call_id,
                descriptor,
                time_ns: world.mono_nanos(),
            },
        );
        if let HookDecision::Deny { message } = decision {
            return Err(RuntimeError::Internal { message }.boxed());
        }

        Ok(call_id)
    }

    /// Evaluate post-call scenario hooks for one binding invocation.
    pub(crate) fn on_after_binding(
        &self,
        world: &mut WorldState,
        descriptor: BindingDescriptor,
        call_id: ScenarioCallId,
    ) {
        self.on_scenario_event(
            world,
            HookEvent::BindingAfter {
                worker_id: self.worker_id,
                call_id,
                descriptor,
                time_ns: world.mono_nanos(),
            },
        );
    }

    /// Evaluate scenario hooks for one scheduler enqueue event.
    pub(crate) fn on_scheduler_enqueue(&self, world: &mut WorldState) {
        self.on_scenario_event(
            world,
            HookEvent::SchedulerEnqueue {
                worker_id: self.worker_id,
                time_ns: world.mono_nanos(),
            },
        );
    }

    /// Evaluate scenario hooks for one scheduler dequeue event.
    pub(crate) fn on_scheduler_dequeue(&self, world: &mut WorldState) {
        self.on_scenario_event(
            world,
            HookEvent::SchedulerDequeue {
                worker_id: self.worker_id,
                time_ns: world.mono_nanos(),
            },
        );
    }

    /// Evaluate scenario hooks for one scheduler timer fire event.
    pub(crate) fn on_scheduler_timer_fire(&self, world: &mut WorldState) {
        self.on_scenario_event(
            world,
            HookEvent::SchedulerTimerFire {
                worker_id: self.worker_id,
                time_ns: world.mono_nanos(),
            },
        );
    }

    /// Evaluate scenario hooks for one ingress enqueue.
    pub(crate) fn on_ingress_enqueue(&self, world: &mut WorldState) {
        self.on_scenario_event(
            world,
            HookEvent::IngressEnqueue {
                worker_id: self.worker_id,
                time_ns: world.mono_nanos(),
            },
        );
    }

    /// Evaluate scenario hooks for one time read.
    pub(crate) fn on_time_read(&self, world: &mut WorldState) {
        self.on_scenario_event(
            world,
            HookEvent::TimeRead {
                worker_id: self.worker_id,
                time_ns: world.mono_nanos(),
            },
        );
    }

    /// Evaluate scenario hooks for one random read.
    pub(crate) fn on_random_read(&self, world: &mut WorldState) {
        self.on_scenario_event(
            world,
            HookEvent::RandomRead {
                worker_id: self.worker_id,
                time_ns: world.mono_nanos(),
            },
        );
    }

    /// Return the total number of deferred scenario faults.
    pub fn deferred_fault_count(&self) -> u64 {
        self.deferred_faults.load(Ordering::Relaxed)
    }

    /// Evaluate one scenario event.
    fn on_scenario_event(&self, world: &mut WorldState, event: HookEvent) -> HookDecision {
        // apply callback hook interceptors first
        let hook_decision = self.run_hook_callbacks(&event);
        if matches!(hook_decision, HookDecision::Deny { .. }) {
            return hook_decision;
        }

        // trigger scenario faults after callback interception
        let faults = match world.decide_scenario(
            self.mode,
            &self.conditions,
            self.runtime_id,
            self.worker_id,
            &event,
        ) {
            Ok(faults) => faults,
            Err(error) => {
                return HookDecision::Deny {
                    message: format!("scenario evaluation failed: {error}"),
                };
            }
        };

        self.apply_triggered_faults(&faults);
        hook_decision
    }

    /// Apply triggered faults for one scenario event.
    fn apply_triggered_faults(&self, triggered_faults: &[TriggeredFault]) {
        // short circuit when no actions fired
        if triggered_faults.is_empty() {
            return;
        }

        // apply each accepted rule
        for triggered_fault in triggered_faults {
            self.apply_triggered_fault(triggered_fault);
        }
    }

    /// Apply one triggered fault to host or simulation state.
    fn apply_triggered_fault(&self, triggered_fault: &TriggeredFault) {
        match &triggered_fault.fault.target {
            FaultTarget::Call {} => self.apply_call_fault(triggered_fault),
            FaultTarget::Entity { kind, .. } => {
                self.apply_entity_fault(kind.as_str(), triggered_fault)
            }
            FaultTarget::Edge { kind, .. } => self.apply_edge_fault(kind.as_str(), triggered_fault),
        }
    }

    /// Apply one call-target fault.
    fn apply_call_fault(&self, _triggered_fault: &TriggeredFault) {
        // NOTE #Incomplete: call faults need binding interception
        self.defer_fault();
    }

    /// Apply one entity-target fault.
    fn apply_entity_fault(&self, _kind: &str, _triggered_fault: &TriggeredFault) {
        // NOTE #Incomplete: entity faults need simulation handlers
        self.defer_fault();
    }

    /// Apply one edge-target fault.
    fn apply_edge_fault(&self, _kind: &str, _triggered_fault: &TriggeredFault) {
        // NOTE #Incomplete: edge faults need simulation handlers
        self.defer_fault();
    }

    /// Defer one scenario fault for a later scenario executor.
    fn defer_fault(&self) {
        self.deferred_faults.fetch_add(1, Ordering::Relaxed);
    }

    /// Run all matching user callbacks for one hook event.
    fn run_hook_callbacks(&self, event: &HookEvent) -> HookDecision {
        let registry = self.registry.read();

        registry.run_callbacks(event)
    }
}

/// Match one text value against one glob pattern.
fn glob_match(pattern: &str, text: &str) -> bool {
    glob_matches(pattern.as_bytes(), 0, text.as_bytes(), 0)
}

#[cfg(test)]
mod tests {
    use destack_engine as engine;
    use destack_workspace::RuntimeOptions;

    use super::{HookDecision, HookSelector};
    use crate::diagnostic::RuntimeError;
    use crate::host::binding::{
        BindingAffinity, BindingDescriptor, BindingDeterminism, BindingProvider, BindingReplayKind,
        BindingReplayPayload,
    };
    use crate::host::{HostSession, default_compile_target_host};
    use crate::runtime::tests::{TestEngine, binding_call_context, runtime_shared_heap};
    use crate::runtime::{Worker, WorkerOptions};
    use crate::world::World;

    /// Ensures before-binding callbacks can deny one matching binding call.
    #[test]
    fn test_on_before_binding_callback_can_deny_matching_call() {
        let options = RuntimeOptions::default();
        let mut world = World::from_options(&options).expect("hook test world should build");
        let shared = runtime_shared_heap(&world, &options);
        let mut worker = Worker::new_in_world(
            Vec::new(),
            &options,
            &mut world.state,
            &shared,
            &engine::StaticSpace::empty(),
            WorkerOptions::default(),
            TestEngine::default(),
        )
        .expect("worker should construct in world");
        let host = HostSession::new(default_compile_target_host(), worker.runtime_id);

        // register one deny callback for matching binding names
        let callback_id =
            worker
                .hooks
                .on_before(HookSelector::binding("destack.test.hook.*"), |_event| {
                    HookDecision::Deny {
                        message: "blocked by callback".to_string(),
                    }
                });

        let descriptor = BindingDescriptor::new(
            "destack.test.hook.block",
            "()",
            BindingDeterminism::Pure,
            BindingReplayKind::BindingCall,
            BindingReplayPayload::Results,
            &[],
            BindingProvider::Runtime,
            BindingAffinity::None,
        );

        // matching binding calls should fail with the callback message
        let error = {
            let world_state = &mut world.state;
            let call_context = binding_call_context(&mut worker, &host, world_state);

            call_context
                .on_before_binding(descriptor)
                .expect_err("matching call should be denied")
        };
        assert!(matches!(
            error.as_ref(),
            RuntimeError::Internal { message } if message == "blocked by callback"
        ));

        // unregistering the callback should restore allow behavior
        assert!(worker.hooks.off(callback_id));
        let is_allowed = {
            let world_state = &mut world.state;
            let call_context = binding_call_context(&mut worker, &host, world_state);

            call_context.on_before_binding(descriptor).is_ok()
        };
        assert!(is_allowed);
    }

    /// Ensures callback selectors only apply to matching binding names.
    #[test]
    fn test_on_before_binding_respects_hook_selector_binding_glob() {
        let options = RuntimeOptions::default();
        let mut world = World::from_options(&options).expect("hook test world should build");
        let shared = runtime_shared_heap(&world, &options);
        let mut worker = Worker::new_in_world(
            Vec::new(),
            &options,
            &mut world.state,
            &shared,
            &engine::StaticSpace::empty(),
            WorkerOptions::default(),
            TestEngine::default(),
        )
        .expect("worker should construct in world");
        let host = HostSession::new(default_compile_target_host(), worker.runtime_id);

        // register one deny callback with one non-matching binding pattern
        let callback_id = worker.hooks.on_before(
            HookSelector::binding("destack.test.hook.nonmatching.*"),
            |_event| HookDecision::Deny {
                message: "blocked by callback".to_string(),
            },
        );

        let descriptor = BindingDescriptor::new(
            "destack.test.hook.allowed",
            "()",
            BindingDeterminism::Pure,
            BindingReplayKind::BindingCall,
            BindingReplayPayload::Results,
            &[],
            BindingProvider::Runtime,
            BindingAffinity::None,
        );

        // non-matching binding calls should continue normally
        let is_allowed = {
            let world_state = &mut world.state;
            let call_context = binding_call_context(&mut worker, &host, world_state);

            call_context.on_before_binding(descriptor).is_ok()
        };
        assert!(is_allowed);

        // unregister should remove exactly one callback
        assert!(worker.hooks.off(callback_id));
    }
}
