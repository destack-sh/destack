use parking_lot::RwLock;

use crate::diagnostic::RuntimeResult;
use crate::platform::ResourceId;
use crate::runtime::bindings::{BindingDescriptor, BindingEngine};
use crate::runtime::random::RandomStreamId;
use crate::runtime::scheduler::{MicrotaskId, TaskId};
use crate::runtime::world::World;
use destack_workspace::{ExecutionMode, RuntimeOptions};

use super::{Hook, Policy, PolicyPlan};

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
    /// External event count for scheduler wake hooks.
    pub external_event_count: Option<usize>,
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

/// Runtime hook dispatch and effect state.
#[derive(Debug)]
pub struct Hooks {
    /// Agent identifier for selector matching.
    agent_id: u64,
    /// Shared world for policy trigger counters.
    world: World,
    /// Execution mode used for rule matching.
    mode: ExecutionMode,
    /// Immutable runtime rule plan.
    plan: RwLock<PolicyPlan>,
}

impl Hooks {
    /// Create runtime hooks from runtime options.
    pub fn from_runtime_options(options: &RuntimeOptions) -> Self {
        Self::from_runtime_options_and_policy_in_world(
            options,
            &Policy::default(),
            World::default(),
            0,
        )
    }

    /// Create runtime hooks from runtime options and one explicit policy.
    pub fn from_runtime_options_and_policy(options: &RuntimeOptions, policy: &Policy) -> Self {
        let world = World::default();
        world.set_policy(policy.clone());

        Self::from_runtime_options_and_policy_in_world(options, policy, world, 0)
    }

    /// Create runtime hooks from runtime options, one explicit policy, and one shared world.
    pub fn from_runtime_options_and_policy_in_world(
        options: &RuntimeOptions,
        policy: &Policy,
        world: World,
        agent_id: u64,
    ) -> Self {
        // compile the immutable policy plan
        let plan = PolicyPlan::from_policy(options.execution, policy);

        Self {
            agent_id,
            world,
            mode: options.execution,
            plan: RwLock::new(plan),
        }
    }

    /// Replace active hook policy.
    pub fn apply_policy(&self, policy: &Policy) {
        // rebuild the immutable policy plan from the active policy
        let plan = PolicyPlan::from_policy(self.mode, policy);

        // replace plan atomically enough for runtime usage
        *self.plan.write() = plan;
    }

    /// Evaluate pre-call runtime effects for one binding invocation.
    pub fn on_before_binding(
        &self,
        descriptor: BindingDescriptor,
        state: HookState,
    ) -> RuntimeResult<()> {
        // count this call for diagnostics and future trigger state
        self.world.record_policy_call();
        self.on_hook(Hook::BindingBefore, Some(descriptor), state);

        Ok(())
    }

    /// Evaluate post-call runtime effects for one binding invocation.
    pub fn on_after_binding(&self, descriptor: BindingDescriptor, state: HookState) {
        self.on_hook(Hook::BindingAfter, Some(descriptor), state);
    }

    /// Evaluate runtime effects for one scheduler enqueue event.
    pub fn on_scheduler_enqueue(&self, state: HookState) {
        self.on_hook(Hook::SchedulerEnqueue, None, state);
    }

    /// Evaluate runtime effects for one scheduler dequeue event.
    pub fn on_scheduler_dequeue(&self, state: HookState) {
        self.on_hook(Hook::SchedulerDequeue, None, state);
    }

    /// Evaluate runtime effects for one scheduler timer fire event.
    pub fn on_scheduler_timer_fire(&self, state: HookState) {
        self.on_hook(Hook::SchedulerTimerFire, None, state);
    }

    /// Evaluate runtime effects for one scheduler wakeup event.
    pub fn on_scheduler_event_wake(&self, state: HookState) {
        self.on_hook(Hook::SchedulerEventWake, None, state);
    }

    /// Evaluate runtime effects for one time read.
    pub fn on_time_read(&self, state: HookState) {
        self.on_hook(Hook::TimeRead, None, state);
    }

    /// Evaluate runtime effects for one random read.
    pub fn on_random_read(&self, state: HookState) {
        self.on_hook(Hook::RandomRead, None, state);
    }

    /// Evaluate runtime effects for one resource attach.
    pub fn on_resource_attach(&self, state: HookState) {
        self.on_hook(Hook::ResourceAttach, None, state);
    }

    /// Evaluate runtime effects for one resource detach.
    pub fn on_resource_detach(&self, state: HookState) {
        self.on_hook(Hook::ResourceDetach, None, state);
    }

    /// Return the number of configured rules.
    pub fn rule_count(&self) -> usize {
        self.plan.read().rule_count()
    }

    /// Evaluate one effect hook.
    fn on_hook(&self, hook: Hook, descriptor: Option<BindingDescriptor>, state: HookState) {
        // count all matching fault actions for future execution wiring
        let indexes = self.plan.read().matching_effect_rule_indexes(
            descriptor,
            state.engine,
            Some(self.agent_id),
            hook,
        );

        if !indexes.is_empty() {
            // NOTE #Incomplete: activation, lifetime, and trigger semantics are not wired yet
            self.world
                .record_policy_matches_for_agent(self.agent_id, &indexes);
        }
    }
}
