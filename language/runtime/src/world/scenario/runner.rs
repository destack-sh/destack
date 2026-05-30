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
}

impl std::fmt::Debug for ScenarioRunner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScenarioRunner")
            .field("runtime_id", &self.runtime_id)
            .field("worker_id", &self.worker_id)
            .field("mode", &self.mode)
            .field("conditions", &self.conditions)
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
        }
    }

    /// Return execution mode for scenario matching.
    pub(crate) fn execution_mode(&self) -> ExecutionMode {
        self.mode
    }

    /// Capture one durable scenario runner snapshot.
    pub(crate) fn snapshot(&self) -> ScenarioRunnerSnapshot {
        ScenarioRunnerSnapshot {
            next_call_id: self.next_call_id.load(Ordering::Relaxed),
        }
    }

    /// Fork one quiescent scenario runner for one child worker.
    pub(crate) fn fork(&self) -> Self {
        let snapshot = self.snapshot();

        let forked = Self::new(
            self.runtime_id,
            self.worker_id,
            self.mode,
            self.conditions.clone(),
        );
        forked.restore_snapshot(&snapshot);

        forked
    }

    /// Restore one durable scenario runner snapshot.
    pub(crate) fn restore_snapshot(&self, snapshot: &ScenarioRunnerSnapshot) {
        self.next_call_id
            .store(snapshot.next_call_id, Ordering::Relaxed);
    }

    /// Evaluate pre-call scenario events for one binding invocation.
    pub(crate) fn on_before_binding(
        &self,
        world: &mut WorldState,
        descriptor: BindingDescriptor,
    ) -> RuntimeResult<ScenarioCallId> {
        // allocate one call id for before and after correlation
        let call_id = self
            .next_call_id
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map(ScenarioCallId)
            .map_err(|_| {
                RuntimeError::Internal {
                    message: "scenario call identifier space exhausted".to_string(),
                }
                .boxed()
            })?;

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

    /// Evaluate one runtime event.
    fn on_runtime_event(&self, world: &mut WorldState, event: RuntimeEvent) -> RuntimeResult<()> {
        let faults = world.decide_scenario(
            self.mode,
            &self.conditions,
            self.runtime_id,
            self.worker_id,
            &event,
        )?;

        self.apply_triggered_faults(&faults)
    }

    /// Apply triggered faults for one runtime event.
    fn apply_triggered_faults(&self, triggered_faults: &[TriggeredFault]) -> RuntimeResult<()> {
        for triggered_fault in triggered_faults {
            self.apply_triggered_fault(triggered_fault)?;
        }

        Ok(())
    }

    /// Apply one triggered fault to host or simulation state.
    fn apply_triggered_fault(&self, triggered_fault: &TriggeredFault) -> RuntimeResult<()> {
        let target = match &triggered_fault.fault.target {
            FaultTarget::Call {} => "call".to_string(),
            FaultTarget::Entity { kind, .. } => format!("entity:{kind}"),
            FaultTarget::Edge { kind, .. } => format!("edge:{kind}"),
        };

        Err(RuntimeError::Internal {
            message: format!(
                "scenario rule {} produced unsupported fault target {target}",
                triggered_fault.rule_id.0
            ),
        }
        .boxed())
    }
}
