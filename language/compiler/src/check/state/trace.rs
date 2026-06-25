use destack_artifact::{ArtifactEvent, ArtifactEventLog};
use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    CheckState, ConstraintId, Dependency, DumpContext, ObligationId, Task, Widening,
};

/// Derived size counters for one checked component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct CheckStats {
    /// The number of allocated variables.
    pub(in crate::check) variables: usize,
    /// The number of collected constraints.
    pub(in crate::check) constraints: usize,
    /// The number of collected obligations.
    pub(in crate::check) obligations: usize,
    /// The number of allocated working types.
    pub(in crate::check) types: usize,
    /// The number of solved variables.
    pub(in crate::check) solutions: usize,
    /// The total number of bounds.
    pub(in crate::check) bounds: usize,
    /// The number of node decisions.
    pub(in crate::check) decisions: usize,
}

/// The bounds visible when one variable event was recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) struct VariableBounds {
    /// Types that must be assignable to the variable.
    pub(in crate::check) lower: SmallVec<[dir::GlobalTypeId; 2]>,
    /// Types the variable must be assignable to.
    pub(in crate::check) upper: SmallVec<[dir::GlobalTypeId; 2]>,
    /// The default solution applied when no bounds arrive.
    pub(in crate::check) default: Option<dir::GlobalTypeId>,
}

/// One event emitted by check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum CheckEvent {
    /// One variable was allocated.
    VariableAllocated {
        /// The allocated variable.
        variable: dir::TypeVariableId,
        /// The literal widening policy applied when solving.
        widening: Widening,
    },
    /// The solver started.
    SolveStarted {
        /// The number of queued tasks.
        tasks: usize,
        /// The number of variables present before solving.
        variables: usize,
    },
    /// One solver task ran once.
    TaskRan {
        /// The zero-based step index.
        step: usize,
        /// The task that ran.
        task: Task,
    },
    /// The solver reached an empty queue.
    SolveFinished {
        /// The number of iterations run.
        iterations: usize,
        /// The number of variables present after solving.
        variables: usize,
    },
    /// One relation constraint was checked or parked.
    RelationChecked {
        /// The constraint.
        constraint: ConstraintId,
        /// Whether the constraint finished.
        is_finished: bool,
    },
    /// One obligation was checked or parked.
    ObligationChecked {
        /// The obligation.
        obligation: ObligationId,
        /// Whether the obligation finished.
        is_finished: bool,
    },
    /// One node was decided.
    NodeDecided {
        /// The decided node.
        node: dir::GlobalNodeIdAny,
    },
    /// One variable was solved.
    VariableSolved {
        /// The solved variable.
        variable: dir::TypeVariableId,
        /// The bounds present when the variable solved.
        bounds: VariableBounds,
        /// The solution type.
        solution: dir::GlobalTypeId,
        /// The number of tasks woken by this solution.
        waiters: usize,
    },
    /// One variable solve is blocked on open dependencies.
    VariableBlocked {
        /// The blocked variable.
        variable: dir::TypeVariableId,
        /// The bounds present when the solve blocked.
        bounds: VariableBounds,
        /// The dependencies blocking the solve.
        blockers: SmallVec<[Dependency; 2]>,
    },
    /// One variable solve had no usable bounds.
    VariableUnsolved {
        /// The unsolved variable.
        variable: dir::TypeVariableId,
        /// The bounds present when the solve stayed open.
        bounds: VariableBounds,
    },
    /// Two open variables were aliased.
    VariableAliased {
        /// The aliased variable.
        variable: dir::TypeVariableId,
        /// The new representative.
        representative: dir::TypeVariableId,
    },
}

impl CheckState<'_> {
    /// Record one check event.
    pub(in crate::check) fn record_event(&mut self, event: CheckEvent) {
        if !self.emit_events {
            return;
        }

        self.events.push(event);
    }

    /// Return rendered event rows for this component.
    pub(in crate::check) fn events(&self) -> ArtifactEventLog {
        let context = DumpContext::new(self);
        let mut log = ArtifactEventLog::new();

        // summarize retained trace state
        log.push(
            ArtifactEvent::new("trace.summary")
                .info()
                .usize("events", self.events.len()),
        );

        // render retained events in order
        for event in &self.events {
            event.render(&context, &mut log);
        }

        log
    }

    /// Return derived size counters for this component.
    pub(in crate::check) fn stats(&self) -> CheckStats {
        let mut bounds = 0;
        let mut solutions = 0;
        for (_, state) in self.solver.variables() {
            bounds += state.lower.len() + state.upper.len();
            solutions += usize::from(state.solution.is_some());
        }
        let types = self
            .modules
            .values()
            .map(|module| module.types.iter_type_ids().count())
            .sum();

        CheckStats {
            variables: self.solver.variable_count(),
            constraints: self.solver.constraint_count(),
            obligations: self.solver.obligation_count(),
            types,
            solutions,
            bounds,
            decisions: self.solver.decision_count(),
        }
    }
}

impl CheckStats {
    /// Render these stats as stable metadata lines.
    pub(in crate::check) fn render_metadata(self) -> String {
        format!(
            "\
check.stats.solve.variables={}
check.stats.solve.types={}
check.stats.solve.constraints={}
check.stats.solve.obligations={}
check.stats.solve.solutions={}
check.stats.solve.bounds={}
check.stats.solve.decisions={}",
            self.variables,
            self.types,
            self.constraints,
            self.obligations,
            self.solutions,
            self.bounds,
            self.decisions,
        )
    }
}
