use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::ResourceId;
use crate::runtime::AgentId;
use crate::runtime::bindings::{BindingDescriptor, BindingEngine};
use crate::runtime::random::RandomStreamId;
use crate::runtime::scheduler::{MicrotaskId, TaskId};
use crate::runtime::world::World;
use destack_source::matches as glob_matches;
use destack_workspace::ExecutionMode;

use super::{PolicyDecision, PolicyIdentity};

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
    /// Trigger when one host event is enqueued into the agent loop.
    HostEventEnqueue,
    /// Trigger when time is read.
    TimeRead,
    /// Trigger when random data is read.
    RandomRead,
    /// Trigger when one resource is attached.
    ResourceAttach,
    /// Trigger when one resource is detached.
    ResourceDetach,
}

/// Selector clauses for hook callback matching.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct HookSelector {
    /// Glob selector for one binding name.
    pub binding: Option<String>,
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
pub(crate) type HookCallback = Arc<dyn Fn(&HookEvent) -> HookDecision + Send + Sync + 'static>;

/// Hook state payload for one runtime hook callback.
#[derive(Debug, Clone, Copy, Default)]
pub struct HookState {
    /// Engine kind for this hook.
    pub engine: Option<BindingEngine>,
    /// Task identifier for scheduler hooks.
    pub task_id: Option<TaskId>,
    /// Microtask identifier for scheduler hooks.
    pub microtask_id: Option<MicrotaskId>,
    /// Random stream identifier for random hooks.
    pub random_stream_id: Option<RandomStreamId>,
    /// Resource identifier for resource hooks.
    pub resource_id: Option<ResourceId>,
}

impl HookState {
    /// Create empty hook state.
    pub const fn empty() -> Self {
        Self {
            engine: None,
            task_id: None,
            microtask_id: None,
            random_stream_id: None,
            resource_id: None,
        }
    }

    /// Create hook state with one engine.
    pub const fn from_engine(engine: Option<BindingEngine>) -> Self {
        Self {
            engine,
            task_id: None,
            microtask_id: None,
            random_stream_id: None,
            resource_id: None,
        }
    }
}

/// Hook event payload emitted by one runtime hook point.
#[derive(Debug, Clone, Copy)]
pub struct HookEvent {
    /// Hook point for this event.
    pub hook: Hook,
    /// Agent identifier for this event.
    pub agent_id: AgentId,
    /// Binding metadata when this event originated from one binding.
    pub descriptor: Option<BindingDescriptor>,
    /// Per-hook metadata payload.
    pub state: HookState,
    /// Whether this event is one binding call event.
    pub is_call_event: bool,
    /// Virtual timestamp for this event.
    pub virtual_time_ns: u64,
}

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
        if callback.hook != event.hook {
            return false;
        }

        let Some(binding_pattern) = &callback.selector.binding else {
            return true;
        };
        let Some(descriptor) = event.descriptor else {
            return false;
        };

        glob_match(binding_pattern, descriptor.name)
    }
}

/// Runtime hook dispatch and effect state.
#[derive(Debug)]
pub struct Hooks {
    /// Agent identifier for selector matching.
    agent_id: AgentId,
    /// Runtime and agent identity for selector matching.
    identity: PolicyIdentity,
    /// Shared world for policy trigger counters.
    world: Arc<World>,
    /// Execution mode used for rule matching.
    mode: ExecutionMode,
    /// Callback-style hook registry.
    registry: RwLock<HookRegistry>,
}

impl Hooks {
    /// Create runtime hooks for one agent in one world.
    pub(crate) fn new(
        world: Arc<World>,
        agent_id: AgentId,
        identity: PolicyIdentity,
        mode: ExecutionMode,
    ) -> Self {
        Self {
            agent_id,
            identity,
            world,
            mode,
            registry: RwLock::new(HookRegistry::default()),
        }
    }

    /// Return runtime and agent identity for policy matching.
    pub(crate) fn policy_identity(&self) -> &PolicyIdentity {
        &self.identity
    }

    /// Return execution mode for policy matching.
    pub(crate) fn execution_mode(&self) -> ExecutionMode {
        self.mode
    }

    /// Register one callback for one hook point.
    pub fn on(&self, hook: Hook, selector: HookSelector, callback: HookCallback) -> HookCallbackId {
        let mut registry = self
            .registry
            .write()
            .unwrap_or_else(|poisoned_lock| poisoned_lock.into_inner());

        registry.register(hook, selector, callback)
    }

    /// Remove one callback by id.
    pub fn off(&self, callback_id: HookCallbackId) -> bool {
        let mut registry = self
            .registry
            .write()
            .unwrap_or_else(|poisoned_lock| poisoned_lock.into_inner());

        registry.unregister(callback_id)
    }

    /// Evaluate pre-call runtime effects for one binding invocation.
    pub fn on_before_binding(
        &self,
        descriptor: BindingDescriptor,
        state: HookState,
    ) -> RuntimeResult<()> {
        let decision = self.on_hook(Hook::BindingBefore, Some(descriptor), state, true);
        if let HookDecision::Deny { message } = decision {
            return Err(RuntimeError::Internal { message }.boxed());
        }

        Ok(())
    }

    /// Evaluate post-call runtime effects for one binding invocation.
    pub fn on_after_binding(&self, descriptor: BindingDescriptor, state: HookState) {
        self.on_hook(Hook::BindingAfter, Some(descriptor), state, false);
    }

    /// Evaluate runtime effects for one scheduler enqueue event.
    pub fn on_scheduler_enqueue(&self, state: HookState) {
        self.on_hook(Hook::SchedulerEnqueue, None, state, false);
    }

    /// Evaluate runtime effects for one scheduler dequeue event.
    pub fn on_scheduler_dequeue(&self, state: HookState) {
        self.on_hook(Hook::SchedulerDequeue, None, state, false);
    }

    /// Evaluate runtime effects for one scheduler timer fire event.
    pub fn on_scheduler_timer_fire(&self, state: HookState) {
        self.on_hook(Hook::SchedulerTimerFire, None, state, false);
    }

    /// Evaluate runtime effects for one host event enqueue.
    pub fn on_host_event_enqueue(&self, state: HookState) {
        self.on_hook(Hook::HostEventEnqueue, None, state, false);
    }

    /// Evaluate runtime effects for one time read.
    pub fn on_time_read(&self, state: HookState) {
        self.on_hook(Hook::TimeRead, None, state, false);
    }

    /// Evaluate runtime effects for one random read.
    pub fn on_random_read(&self, state: HookState) {
        self.on_hook(Hook::RandomRead, None, state, false);
    }

    /// Evaluate runtime effects for one resource attach.
    pub fn on_resource_attach(&self, state: HookState) {
        self.on_hook(Hook::ResourceAttach, None, state, false);
    }

    /// Evaluate runtime effects for one resource detach.
    pub fn on_resource_detach(&self, state: HookState) {
        self.on_hook(Hook::ResourceDetach, None, state, false);
    }

    /// Return the number of configured rules.
    pub fn rule_count(&self) -> usize {
        self.world.policy_rule_count()
    }

    /// Evaluate one effect hook.
    fn on_hook(
        &self,
        hook: Hook,
        descriptor: Option<BindingDescriptor>,
        state: HookState,
        is_call_event: bool,
    ) -> HookDecision {
        let event = HookEvent {
            hook,
            agent_id: self.agent_id,
            descriptor,
            state,
            is_call_event,
            virtual_time_ns: self.world.clock().mono_nanos(),
        };

        let hook_decision = self.dispatch_hook_event(&event);

        // NOTE #Incomplete: fault execution wiring is pending, this only updates trigger state
        let decisions = self
            .world
            .evaluate_policy_event(self.mode, &self.identity, &event);

        self.apply_policy_decisions(&decisions);
        hook_decision
    }

    /// Apply policy decisions for one hook event.
    fn apply_policy_decisions(&self, decisions: &[PolicyDecision]) {
        // short circuit when no effects fired
        if decisions.is_empty() {
            return;
        }

        // NOTE #Incomplete: execute decisions through host and simulation backends
        let _ = decisions;
    }

    /// Dispatch one hook event to all matching user callbacks.
    fn dispatch_hook_event(&self, event: &HookEvent) -> HookDecision {
        let registry = self
            .registry
            .read()
            .unwrap_or_else(|poisoned_lock| poisoned_lock.into_inner());

        registry.dispatch(event)
    }
}

/// Match one text value against one glob pattern.
fn glob_match(pattern: &str, text: &str) -> bool {
    glob_matches(pattern.as_bytes(), 0, text.as_bytes(), 0)
}
