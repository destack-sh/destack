use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::Mutex;

use crate::diagnostic::RuntimeResult;
use crate::platform::ResourceId;
use crate::runtime::bindings::{BindingDescriptor, BindingEngine};
use crate::runtime::random::RandomStreamId;
use crate::runtime::scheduler::{MicrotaskId, TaskId};
use destack_workspace::{RuntimeHook, RuntimeOptions};

use super::RuntimeRulePlan;

/// Hook state payload for one runtime hook callback.
#[derive(Debug, Clone, Copy, Default)]
pub struct RuntimeHookState {
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
    /// External event count for scheduler wake hooks.
    pub external_event_count: Option<usize>,
}

impl RuntimeHookState {
    /// Create empty hook state.
    pub const fn empty() -> Self {
        Self {
            engine: None,
            task_id: None,
            microtask_id: None,
            random_stream_id: None,
            resource_id: None,
            external_event_count: None,
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
            external_event_count: None,
        }
    }
}

/// Runtime rules and effect state.
#[derive(Debug)]
pub struct RuntimeRules {
    /// Immutable runtime rule plan.
    plan: RuntimeRulePlan,
    /// Total binding calls observed.
    calls_seen: AtomicU64,
    /// Matched effect actions per rule index.
    matched_effects_seen: Mutex<Vec<u64>>,
}

impl RuntimeRules {
    /// Create runtime rules from runtime options.
    pub fn from_runtime_options(options: &RuntimeOptions) -> Self {
        // compile the immutable rule plan
        let plan = RuntimeRulePlan::from_runtime_options(options);
        let counters = vec![0; plan.rule_count()];

        Self {
            plan,
            calls_seen: AtomicU64::new(0),
            matched_effects_seen: Mutex::new(counters),
        }
    }

    /// Evaluate pre-call runtime effects for one binding invocation.
    pub fn on_before_binding(
        &self,
        descriptor: BindingDescriptor,
        state: RuntimeHookState,
    ) -> RuntimeResult<()> {
        // count this call for diagnostics and future trigger state
        self.calls_seen.fetch_add(1, Ordering::Relaxed);
        self.on_hook_site(RuntimeHook::BindingBefore, Some(descriptor), state);

        Ok(())
    }

    /// Evaluate post-call runtime effects for one binding invocation.
    pub fn on_after_binding(&self, descriptor: BindingDescriptor, state: RuntimeHookState) {
        self.on_hook_site(RuntimeHook::BindingAfter, Some(descriptor), state);
    }

    /// Evaluate runtime effects for one scheduler enqueue event.
    pub fn on_scheduler_enqueue(&self, state: RuntimeHookState) {
        self.on_hook_site(RuntimeHook::SchedulerEnqueue, None, state);
    }

    /// Evaluate runtime effects for one scheduler dequeue event.
    pub fn on_scheduler_dequeue(&self, state: RuntimeHookState) {
        self.on_hook_site(RuntimeHook::SchedulerDequeue, None, state);
    }

    /// Evaluate runtime effects for one scheduler timer fire event.
    pub fn on_scheduler_timer_fire(&self, state: RuntimeHookState) {
        self.on_hook_site(RuntimeHook::SchedulerTimerFire, None, state);
    }

    /// Evaluate runtime effects for one scheduler wakeup event.
    pub fn on_scheduler_event_wake(&self, state: RuntimeHookState) {
        self.on_hook_site(RuntimeHook::SchedulerEventWake, None, state);
    }

    /// Evaluate runtime effects for one time read.
    pub fn on_time_read(&self, state: RuntimeHookState) {
        self.on_hook_site(RuntimeHook::TimeRead, None, state);
    }

    /// Evaluate runtime effects for one random read.
    pub fn on_random_read(&self, state: RuntimeHookState) {
        self.on_hook_site(RuntimeHook::RandomRead, None, state);
    }

    /// Evaluate runtime effects for one resource attach.
    pub fn on_resource_attach(&self, state: RuntimeHookState) {
        self.on_hook_site(RuntimeHook::ResourceAttach, None, state);
    }

    /// Evaluate runtime effects for one resource detach.
    pub fn on_resource_detach(&self, state: RuntimeHookState) {
        self.on_hook_site(RuntimeHook::ResourceDetach, None, state);
    }

    /// Return the number of configured rules.
    pub fn rule_count(&self) -> usize {
        self.plan.rule_count()
    }

    /// Evaluate one effect hook.
    fn on_hook_site(
        &self,
        site: RuntimeHook,
        descriptor: Option<BindingDescriptor>,
        state: RuntimeHookState,
    ) {
        // count the first matching effect action for future execution wiring
        if let Some(index) = self
            .plan
            .first_matching_effect_rule(descriptor, state.engine, site)
        {
            // NOTE #Incomplete: activation, lifetime, and trigger semantics are not wired yet
            let mut matched_effects_seen = self.matched_effects_seen.lock();
            matched_effects_seen[index] = matched_effects_seen[index].saturating_add(1);
        }
    }
}
