use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::host::binding::BindingDescriptor;
use crate::runtime::WorkerId;
use crate::world::{RuntimeId, WorldState};
use destack_workspace::{ConditionSet, ExecutionMode};

use super::{Fault, FaultRuleId, FaultTarget, RuntimeEvent, RuntimeEventKind, ScenarioCallId};

/// Durable scenario runner state captured at one checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScenarioRunnerSnapshot {
    /// The next scenario call identifier to allocate.
    pub next_call_id: u64,
    /// The number of scenario faults deferred to a later scenario executor.
    pub deferred_faults: u64,
}

/// One fault accepted by trigger evaluation for one runtime event.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct TriggeredFault {
    /// Stable identifier of the rule that fired.
    pub rule_id: FaultRuleId,
    /// Event kind that produced this fault.
    pub event: RuntimeEventKind,
    /// Worker identifier for this action.
    pub worker_id: WorkerId,
    /// Binding call identifier when one call event fired this fault.
    pub call_id: Option<ScenarioCallId>,
    /// Fault emitted by this scenario rule.
    pub fault: Fault,
}

/// Per-worker scenario rule runner.
pub struct ScenarioRunner {
    /// Runtime identifier for selector matching.
    runtime_id: RuntimeId,
    /// Worker identifier for selector matching.
    worker_id: WorkerId,
    /// Execution mode used for rule matching.
    mode: ExecutionMode,
    /// Source graph conditions used for rule matching.
    conditions: ConditionSet,
    /// Next scenario call identifier sequence.
    next_call_id: AtomicU64,
    /// Total scenario faults deferred to a later scenario executor.
    deferred_faults: AtomicU64,
}

impl std::fmt::Debug for ScenarioRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let deferred_faults = self.deferred_faults.load(Ordering::Relaxed);

        f.debug_struct("ScenarioRunner")
            .field("runtime_id", &self.runtime_id)
            .field("worker_id", &self.worker_id)
            .field("mode", &self.mode)
            .field("conditions", &self.conditions)
            .field("deferred_faults", &deferred_faults)
            .finish()
    }
}

impl ScenarioRunner {
    /// Create one scenario runner for one worker in one world.
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
        // require no deferred scenario faults
        if self.deferred_faults.load(Ordering::Relaxed) != 0 {
            return Err(RuntimeError::CaptureBarrier {
                component: "runtime.scenario".to_string(),
                mode: "snapshot".to_string(),
                detail: "scenario faults are still pending".to_string(),
            }
            .boxed());
        }

        Ok(())
    }

    /// Capture one durable scenario runner snapshot.
    pub(crate) fn snapshot(&self) -> RuntimeResult<ScenarioRunnerSnapshot> {
        self.capture_barrier()?;

        Ok(ScenarioRunnerSnapshot {
            next_call_id: self.next_call_id.load(Ordering::Relaxed),
            deferred_faults: self.deferred_faults.load(Ordering::Relaxed),
        })
    }

    /// Fork one quiescent scenario runner for one child worker.
    pub(crate) fn try_fork(&self) -> RuntimeResult<Option<Self>> {
        // require one quiescent scenario runner first
        let snapshot = match self.snapshot() {
            Ok(snapshot) => snapshot,
            Err(_) => return Ok(None),
        };

        // rebuild one fresh runner with the same scalar state
        let forked = Self::new(
            self.runtime_id,
            self.worker_id,
            self.mode,
            self.conditions.clone(),
        );
        forked.restore_snapshot(&snapshot)?;

        Ok(Some(forked))
    }

    /// Restore one durable scenario runner snapshot.
    pub(crate) fn restore_snapshot(&self, snapshot: &ScenarioRunnerSnapshot) -> RuntimeResult<()> {
        self.capture_barrier()?;
        self.next_call_id
            .store(snapshot.next_call_id, Ordering::Relaxed);
        self.deferred_faults
            .store(snapshot.deferred_faults, Ordering::Relaxed);

        Ok(())
    }

    /// Evaluate pre-call scenario events for one binding invocation.
    pub(crate) fn on_before_binding(
        &self,
        world: &mut WorldState,
        descriptor: BindingDescriptor,
    ) -> RuntimeResult<ScenarioCallId> {
        // allocate one call id for before and after correlation
        let call_id = ScenarioCallId(self.next_call_id.fetch_add(1, Ordering::Relaxed));

        self.on_runtime_event(
            world,
            RuntimeEvent::BindingBefore {
                worker_id: self.worker_id,
                call_id,
                descriptor,
                time_ns: world.mono_nanos(),
            },
        )?;

        Ok(call_id)
    }

    /// Evaluate post-call scenario events for one binding invocation.
    pub(crate) fn on_after_binding(
        &self,
        world: &mut WorldState,
        descriptor: BindingDescriptor,
        call_id: ScenarioCallId,
    ) -> RuntimeResult<()> {
        self.on_runtime_event(
            world,
            RuntimeEvent::BindingAfter {
                worker_id: self.worker_id,
                call_id,
                descriptor,
                time_ns: world.mono_nanos(),
            },
        )
    }

    /// Evaluate scenario events for one task ready event.
    pub(crate) fn on_task_ready(&self, world: &mut WorldState) -> RuntimeResult<()> {
        self.on_runtime_event(
            world,
            RuntimeEvent::TaskReady {
                worker_id: self.worker_id,
                time_ns: world.mono_nanos(),
            },
        )
    }

    /// Evaluate scenario events for one task start event.
    pub(crate) fn on_task_start(&self, world: &mut WorldState) -> RuntimeResult<()> {
        self.on_runtime_event(
            world,
            RuntimeEvent::TaskStart {
                worker_id: self.worker_id,
                time_ns: world.mono_nanos(),
            },
        )
    }

    /// Evaluate scenario events for one timer fire event.
    pub(crate) fn on_timer_fire(&self, world: &mut WorldState) -> RuntimeResult<()> {
        self.on_runtime_event(
            world,
            RuntimeEvent::TimerFire {
                worker_id: self.worker_id,
                time_ns: world.mono_nanos(),
            },
        )
    }

    /// Evaluate scenario events for one ingress-ready event.
    pub(crate) fn on_ingress_ready(&self, world: &mut WorldState) -> RuntimeResult<()> {
        self.on_runtime_event(
            world,
            RuntimeEvent::IngressReady {
                worker_id: self.worker_id,
                time_ns: world.mono_nanos(),
            },
        )
    }

    /// Evaluate scenario events for one clock read.
    pub(crate) fn on_clock_read(&self, world: &mut WorldState) -> RuntimeResult<()> {
        self.on_runtime_event(
            world,
            RuntimeEvent::ClockRead {
                worker_id: self.worker_id,
                time_ns: world.mono_nanos(),
            },
        )
    }

    /// Evaluate scenario events for one random read.
    pub(crate) fn on_random_read(&self, world: &mut WorldState) -> RuntimeResult<()> {
        self.on_runtime_event(
            world,
            RuntimeEvent::RandomRead {
                worker_id: self.worker_id,
                time_ns: world.mono_nanos(),
            },
        )
    }

    /// Return the total number of deferred scenario faults.
    pub fn deferred_fault_count(&self) -> u64 {
        self.deferred_faults.load(Ordering::Relaxed)
    }

    /// Evaluate one runtime event.
    fn on_runtime_event(&self, world: &mut WorldState, event: RuntimeEvent) -> RuntimeResult<()> {
        let faults = world.decide_scenario(
            self.mode,
            &self.conditions,
            self.runtime_id,
            self.worker_id,
            &event,
        )?;

        self.apply_triggered_faults(&faults);

        Ok(())
    }

    /// Apply triggered faults for one runtime event.
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
}
