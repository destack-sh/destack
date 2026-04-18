use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{
    ResourceBacking, ResourceCapture, ResourceId, ResourceKind, ResourcePortability,
};
use crate::runtime::WorkerId;
use crate::runtime::bindings::{BindingDescriptor, BindingEngine};
use crate::runtime::observe::Observation;
use crate::runtime::world::{RuntimeId, WorldEntityKind, WorldRef, WorldResource, WorldResourceId};
use destack_source::matches as glob_matches;
use destack_workspace::ExecutionMode;

use super::{Effect, FaultTarget, PolicyDecision};

/// Durable hook state captured at one checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookSnapshot {
    /// The next policy call identifier to allocate.
    pub next_call_id: u64,
    /// The number of unapplied policy decisions.
    pub unapplied_policy_decisions: u64,
}

/// Hook for runtime effect rules.
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

/// Stable identifier for one binding call policy event pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PolicyCallId(pub u64);

/// Policy event payload emitted by one runtime hook point.
#[derive(Debug, Clone, Copy)]
pub enum HookEvent {
    /// Event fired before invoking one binding.
    BindingBefore {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Binding call identifier for before and after correlation.
        call_id: PolicyCallId,
        /// Binding metadata for this event.
        descriptor: BindingDescriptor,
        /// Engine kind for this event.
        engine: Option<BindingEngine>,
        /// Virtual timestamp for this event.
        virtual_time_ns: u64,
    },
    /// Event fired after invoking one binding.
    BindingAfter {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Binding call identifier for before and after correlation.
        call_id: PolicyCallId,
        /// Binding metadata for this event.
        descriptor: BindingDescriptor,
        /// Engine kind for this event.
        engine: Option<BindingEngine>,
        /// Virtual timestamp for this event.
        virtual_time_ns: u64,
    },
    /// Event fired when one task is enqueued.
    SchedulerEnqueue {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Virtual timestamp for this event.
        virtual_time_ns: u64,
    },
    /// Event fired when one task is dequeued.
    SchedulerDequeue {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Virtual timestamp for this event.
        virtual_time_ns: u64,
    },
    /// Event fired when one timer is dispatched.
    SchedulerTimerFire {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Virtual timestamp for this event.
        virtual_time_ns: u64,
    },
    /// Event fired when one ingress event is enqueued.
    IngressEnqueue {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Virtual timestamp for this event.
        virtual_time_ns: u64,
    },
    /// Event fired when time is read.
    TimeRead {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Engine kind for this event.
        engine: Option<BindingEngine>,
        /// Virtual timestamp for this event.
        virtual_time_ns: u64,
    },
    /// Event fired when random data is read.
    RandomRead {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Engine kind for this event.
        engine: Option<BindingEngine>,
        /// Virtual timestamp for this event.
        virtual_time_ns: u64,
    },
    /// Event fired when one resource is attached.
    ResourceAttach {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Virtual timestamp for this event.
        virtual_time_ns: u64,
    },
    /// Event fired when one resource is detached.
    ResourceDetach {
        /// Worker identifier for this event.
        worker_id: WorkerId,
        /// Virtual timestamp for this event.
        virtual_time_ns: u64,
    },
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
    pub(crate) const fn call_id(&self) -> Option<PolicyCallId> {
        match self {
            Self::BindingBefore { call_id, .. } | Self::BindingAfter { call_id, .. } => {
                Some(*call_id)
            }
            _ => None,
        }
    }

    /// Return one engine kind when present.
    pub(crate) const fn engine(&self) -> Option<BindingEngine> {
        match self {
            Self::BindingBefore { engine, .. }
            | Self::BindingAfter { engine, .. }
            | Self::TimeRead { engine, .. }
            | Self::RandomRead { engine, .. } => *engine,
            _ => None,
        }
    }

    /// Return whether this event increments call-scoped trigger counters.
    pub(crate) const fn counts_as_call_event(&self) -> bool {
        matches!(self, Self::BindingBefore { .. })
    }

    /// Return the virtual timestamp for this event.
    pub(crate) const fn virtual_time_ns(&self) -> u64 {
        match self {
            Self::BindingBefore {
                virtual_time_ns, ..
            }
            | Self::BindingAfter {
                virtual_time_ns, ..
            }
            | Self::SchedulerEnqueue {
                virtual_time_ns, ..
            }
            | Self::SchedulerDequeue {
                virtual_time_ns, ..
            }
            | Self::SchedulerTimerFire {
                virtual_time_ns, ..
            }
            | Self::IngressEnqueue {
                virtual_time_ns, ..
            }
            | Self::TimeRead {
                virtual_time_ns, ..
            }
            | Self::RandomRead {
                virtual_time_ns, ..
            }
            | Self::ResourceAttach {
                virtual_time_ns, ..
            }
            | Self::ResourceDetach {
                virtual_time_ns, ..
            } => *virtual_time_ns,
        }
    }
}

/// Selector clauses for hook callback matching.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct HookSelector {
    /// Glob selector for one binding name.
    pub binding: Option<String>,
    /// Glob selector for one function name.
    pub function: Option<String>,
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
            function: None,
        }
    }

    /// Match one function glob for call hook events.
    pub fn function(pattern: impl Into<String>) -> Self {
        Self {
            binding: None,
            function: Some(pattern.into()),
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

/// Runtime context passed to one custom effect handler callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomEffectInvocation {
    /// Stable rule identifier that produced this effect.
    pub rule_id: String,
    /// Hook that produced this effect.
    pub hook: Hook,
    /// Worker identifier for this effect.
    pub worker_id: WorkerId,
    /// Binding call identifier when this effect comes from one call event.
    pub call_id: Option<PolicyCallId>,
    /// Stable custom effect handler key.
    pub handler: String,
    /// Optional custom effect payload.
    pub payload: Option<String>,
}

/// Stable identifier for one registered hook callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HookCallbackId(pub u64);

/// Hook callback function signature.
pub type HookCallback = Arc<dyn Fn(&HookEvent) -> HookDecision + Send + Sync + 'static>;

/// Custom effect callback function signature.
pub type CustomEffectHandler =
    Arc<dyn Fn(&CustomEffectInvocation) -> RuntimeResult<()> + Send + Sync + 'static>;

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

    /// Dispatch one event to matching callbacks in registration order.
    pub(crate) fn dispatch(&self, event: &HookEvent) -> HookDecision {
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

        if callback.selector.function.is_some() {
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

/// Runtime hook dispatch and effect state.
pub struct Hooks {
    /// Runtime identifier for selector matching.
    runtime_id: RuntimeId,
    /// Worker identifier for selector matching.
    worker_id: WorkerId,
    /// Execution mode used for rule matching.
    mode: ExecutionMode,
    /// Callback-style hook registry.
    registry: RwLock<HookRegistry>,
    /// Custom effect handlers keyed by custom effect kind.
    custom_effect_handlers: RwLock<HashMap<String, CustomEffectHandler>>,
    /// Next policy call identifier sequence.
    next_call_id: AtomicU64,
    /// Total policy decisions accepted but not yet executed.
    unapplied_policy_decisions: AtomicU64,
}

impl std::fmt::Debug for Hooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let callback_count = self.registry.read().callbacks.len();
        let custom_effect_handler_count = self.custom_effect_handlers.read().len();
        let unapplied_policy_decisions = self.unapplied_policy_decisions.load(Ordering::Relaxed);

        f.debug_struct("Hooks")
            .field("runtime_id", &self.runtime_id)
            .field("worker_id", &self.worker_id)
            .field("mode", &self.mode)
            .field("callback_count", &callback_count)
            .field("custom_effect_handler_count", &custom_effect_handler_count)
            .field("unapplied_policy_decisions", &unapplied_policy_decisions)
            .finish()
    }
}

impl Hooks {
    /// Create runtime hooks for one worker in one world.
    pub(crate) fn new(runtime_id: RuntimeId, worker_id: WorkerId, mode: ExecutionMode) -> Self {
        Self {
            runtime_id,
            worker_id,
            mode,
            registry: RwLock::new(HookRegistry::default()),
            custom_effect_handlers: RwLock::new(HashMap::new()),
            next_call_id: AtomicU64::new(1),
            unapplied_policy_decisions: AtomicU64::new(0),
        }
    }

    /// Return execution mode for policy matching.
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

        // require no registered custom effect handlers
        if !self.custom_effect_handlers.read().is_empty() {
            return Err(RuntimeError::CaptureBarrier {
                component: "runtime.hooks".to_string(),
                mode: "snapshot".to_string(),
                detail: "custom effect handlers are still registered".to_string(),
            }
            .boxed());
        }

        // require no unapplied policy decisions
        if self.unapplied_policy_decisions.load(Ordering::Relaxed) != 0 {
            return Err(RuntimeError::CaptureBarrier {
                component: "runtime.hooks".to_string(),
                mode: "snapshot".to_string(),
                detail: "policy decisions are still pending".to_string(),
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
            unapplied_policy_decisions: self.unapplied_policy_decisions.load(Ordering::Relaxed),
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
        let forked = Self::new(self.runtime_id, self.worker_id, self.mode);
        forked.restore_snapshot(&snapshot)?;

        Ok(Some(forked))
    }

    /// Restore one durable hook snapshot.
    pub(crate) fn restore_snapshot(&self, snapshot: &HookSnapshot) -> RuntimeResult<()> {
        self.capture_barrier()?;
        self.next_call_id
            .store(snapshot.next_call_id, Ordering::Relaxed);
        self.unapplied_policy_decisions
            .store(snapshot.unapplied_policy_decisions, Ordering::Relaxed);

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

    /// Register one custom effect callback by handler key.
    pub fn on_custom_effect(
        &self,
        handler: impl Into<String>,
        callback: impl Fn(&CustomEffectInvocation) -> RuntimeResult<()> + Send + Sync + 'static,
    ) {
        let mut handlers = self.custom_effect_handlers.write();

        handlers.insert(handler.into(), Arc::new(callback));
    }

    /// Remove one custom effect callback by handler key.
    pub fn off_custom_effect(&self, handler: &str) -> bool {
        let mut handlers = self.custom_effect_handlers.write();

        handlers.remove(handler).is_some()
    }

    /// Evaluate pre-call runtime effects for one binding invocation.
    pub(crate) fn on_before_binding(
        &self,
        world: &WorldRef,
        descriptor: BindingDescriptor,
        engine: Option<BindingEngine>,
    ) -> RuntimeResult<PolicyCallId> {
        // allocate one call id for before and after correlation
        let call_id = PolicyCallId(self.next_call_id.fetch_add(1, Ordering::Relaxed));

        let decision = self.on_policy_event(
            world,
            HookEvent::BindingBefore {
                worker_id: self.worker_id,
                call_id,
                descriptor,
                engine,
                virtual_time_ns: world.mono_nanos(),
            },
        );
        if let HookDecision::Deny { message } = decision {
            return Err(RuntimeError::Internal { message }.boxed());
        }

        Ok(call_id)
    }

    /// Evaluate post-call runtime effects for one binding invocation.
    pub(crate) fn on_after_binding(
        &self,
        world: &WorldRef,
        descriptor: BindingDescriptor,
        engine: Option<BindingEngine>,
        call_id: PolicyCallId,
    ) {
        self.on_policy_event(
            world,
            HookEvent::BindingAfter {
                worker_id: self.worker_id,
                call_id,
                descriptor,
                engine,
                virtual_time_ns: world.mono_nanos(),
            },
        );
    }

    /// Evaluate runtime effects for one scheduler enqueue event.
    pub(crate) fn on_scheduler_enqueue(&self, world: &WorldRef) {
        self.on_policy_event(
            world,
            HookEvent::SchedulerEnqueue {
                worker_id: self.worker_id,
                virtual_time_ns: world.mono_nanos(),
            },
        );
    }

    /// Evaluate runtime effects for one scheduler dequeue event.
    pub(crate) fn on_scheduler_dequeue(&self, world: &WorldRef) {
        self.on_policy_event(
            world,
            HookEvent::SchedulerDequeue {
                worker_id: self.worker_id,
                virtual_time_ns: world.mono_nanos(),
            },
        );
    }

    /// Evaluate runtime effects for one scheduler timer fire event.
    pub(crate) fn on_scheduler_timer_fire(&self, world: &WorldRef) {
        self.on_policy_event(
            world,
            HookEvent::SchedulerTimerFire {
                worker_id: self.worker_id,
                virtual_time_ns: world.mono_nanos(),
            },
        );
    }

    /// Evaluate runtime effects for one ingress enqueue.
    pub(crate) fn on_ingress_enqueue(&self, world: &WorldRef) {
        self.on_policy_event(
            world,
            HookEvent::IngressEnqueue {
                worker_id: self.worker_id,
                virtual_time_ns: world.mono_nanos(),
            },
        );
    }

    /// Evaluate runtime effects for one time read.
    pub(crate) fn on_time_read(&self, world: &WorldRef, engine: Option<BindingEngine>) {
        self.on_policy_event(
            world,
            HookEvent::TimeRead {
                worker_id: self.worker_id,
                engine,
                virtual_time_ns: world.mono_nanos(),
            },
        );
    }

    /// Evaluate runtime effects for one random read.
    pub(crate) fn on_random_read(&self, world: &WorldRef, engine: Option<BindingEngine>) {
        self.on_policy_event(
            world,
            HookEvent::RandomRead {
                worker_id: self.worker_id,
                engine,
                virtual_time_ns: world.mono_nanos(),
            },
        );
    }

    /// Evaluate runtime effects for one resource attach.
    pub(crate) fn on_resource_attach(
        &self,
        world: &WorldRef,
        resource_id: ResourceId,
        resource_kind: ResourceKind,
        resource_label: Option<&str>,
        resource_backing: ResourceBacking,
        resource_capture: ResourceCapture,
        resource_portability: ResourcePortability,
        _engine: Option<BindingEngine>,
    ) -> RuntimeResult<()> {
        let resource = WorldResource::new(
            WorldResourceId::new(self.worker_id, resource_id),
            WorldEntityKind::from(resource_kind.kind_id()),
            resource_label.map(ToString::to_string),
            resource_backing,
            resource_capture,
            resource_portability,
        );
        world
            .topology_mut()
            .attach_resource(
                resource.id,
                resource.kind.clone(),
                resource.label.as_deref(),
            )
            .map_err(|message| {
                RuntimeError::Internal {
                    message: message.to_string(),
                }
                .boxed()
            })?;
        world.resources_mut().insert(resource.id, resource);
        world.observe(Observation::resource_attached(
            self.worker_id,
            WorldResourceId::new(self.worker_id, resource_id),
            resource_backing,
            resource_capture,
            resource_portability,
        ));

        self.on_policy_event(
            world,
            HookEvent::ResourceAttach {
                worker_id: self.worker_id,
                virtual_time_ns: world.mono_nanos(),
            },
        );

        Ok(())
    }

    /// Evaluate runtime effects for one resource detach.
    pub(crate) fn on_resource_detach(
        &self,
        world: &WorldRef,
        resource_id: ResourceId,
        _resource_kind: ResourceKind,
        resource_label: Option<&str>,
        _engine: Option<BindingEngine>,
    ) -> RuntimeResult<()> {
        let _resource_label = resource_label;
        let world_resource_id = WorldResourceId::new(self.worker_id, resource_id);
        world.topology_mut().detach_resource(world_resource_id);
        world.resources_mut().remove(&world_resource_id);
        world.observe(Observation::resource_detached(
            self.worker_id,
            world_resource_id,
        ));

        self.on_policy_event(
            world,
            HookEvent::ResourceDetach {
                worker_id: self.worker_id,
                virtual_time_ns: world.mono_nanos(),
            },
        );

        Ok(())
    }

    /// Return the total number of unapplied policy decisions.
    pub fn unapplied_policy_decision_count(&self) -> u64 {
        self.unapplied_policy_decisions.load(Ordering::Relaxed)
    }

    /// Evaluate one policy event.
    fn on_policy_event(&self, world: &WorldRef, event: HookEvent) -> HookDecision {
        // apply callback hook interceptors first
        let hook_decision = self.dispatch_hook_event(&event);
        if matches!(hook_decision, HookDecision::Deny { .. }) {
            return hook_decision;
        }

        // TODO #Incomplete: execute policy decisions after trigger evaluation
        let decisions =
            match world.evaluate_policy_event(self.mode, self.runtime_id, self.worker_id, &event) {
                Ok(decisions) => decisions,
                Err(error) => {
                    return HookDecision::Deny {
                        message: format!("policy evaluation failed: {error}"),
                    };
                }
            };

        self.apply_policy_decisions(&decisions);
        hook_decision
    }

    /// Apply policy decisions for one policy event.
    fn apply_policy_decisions(&self, decisions: &[PolicyDecision]) {
        // short circuit when no effects fired
        if decisions.is_empty() {
            return;
        }

        // route each decision through its target dispatch path
        for decision in decisions {
            self.route_policy_decision(decision);
        }
    }

    /// Route one policy decision to host or simulation execution lanes.
    fn route_policy_decision(&self, decision: &PolicyDecision) {
        match &decision.effect {
            Effect::Fault { fault } => match &fault.target {
                FaultTarget::Call {} => self.route_call_fault(decision),
                FaultTarget::Entity { kind, .. } => {
                    self.route_simulation_entity_fault(kind.as_str(), decision)
                }
                FaultTarget::Edge { kind, .. } => {
                    self.route_simulation_edge_fault(kind.as_str(), decision)
                }
            },
            Effect::Custom { custom } => self.route_custom_effect(decision, custom),
            // NOTE #Incomplete: non-fault trigger effects are not wired yet
            _ => self.record_unapplied_policy_decision(),
        }
    }

    /// Route one call-target fault decision.
    fn route_call_fault(&self, _decision: &PolicyDecision) {
        // TODO #Incomplete: execute call-target faults through host binding interception lanes
        self.record_unapplied_policy_decision();
    }

    /// Route one entity-target fault decision.
    fn route_simulation_entity_fault(&self, _kind: &str, _decision: &PolicyDecision) {
        // TODO #Incomplete: execute entity-target faults through simulation entity handlers
        self.record_unapplied_policy_decision();
    }

    /// Route one edge-target fault decision.
    fn route_simulation_edge_fault(&self, _kind: &str, _decision: &PolicyDecision) {
        // TODO #Incomplete: execute edge-target faults through simulation edge handlers
        self.record_unapplied_policy_decision();
    }

    /// Route one custom-effect decision to one registered custom handler.
    fn route_custom_effect(&self, decision: &PolicyDecision, custom: &super::CustomEffect) {
        let handler = {
            let handlers = self.custom_effect_handlers.read();
            handlers.get(custom.handler.as_str()).cloned()
        };
        let Some(handler) = handler else {
            self.record_unapplied_policy_decision();
            return;
        };

        let invocation = CustomEffectInvocation {
            rule_id: decision.rule_id.0.clone(),
            hook: decision.hook,
            worker_id: decision.worker_id,
            call_id: decision.call_id,
            handler: custom.handler.clone(),
            payload: custom.payload.clone(),
        };

        if handler(&invocation).is_err() {
            self.record_unapplied_policy_decision();
        }
    }

    /// Record one policy decision that has not been applied yet.
    fn record_unapplied_policy_decision(&self) {
        // count unapplied decisions until execution wiring lands
        self.unapplied_policy_decisions
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Dispatch one policy event to all matching user callbacks.
    fn dispatch_hook_event(&self, event: &HookEvent) -> HookDecision {
        let registry = self.registry.read();

        registry.dispatch(event)
    }
}

/// Match one text value against one glob pattern.
fn glob_match(pattern: &str, text: &str) -> bool {
    glob_matches(pattern.as_bytes(), 0, text.as_bytes(), 0)
}
