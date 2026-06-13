use destack_artifact::{ArtifactEvent, ArtifactEventLog};
use destack_dir as dir;

use crate::check::{CheckState, ConstraintId, Task};

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

/// One event emitted by check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum CheckEvent {
    /// The solver started.
    SolveStart {
        /// The number of queued tasks.
        tasks: usize,
        /// The number of variables present before solving.
        variables: usize,
    },
    /// One solver task ran once.
    SolveStep {
        /// The zero-based step index.
        step: usize,
        /// The task that ran.
        task: Task,
    },
    /// The solver reached an empty queue.
    SolveFinish {
        /// The number of iterations run.
        iterations: usize,
        /// The number of variables present after solving.
        variables: usize,
    },
    /// One constraint was solved or parked.
    Relate {
        /// The constraint.
        constraint: ConstraintId,
        /// Whether the constraint finished.
        finished: bool,
    },
    /// One node was decided.
    Decide {
        /// The decided node.
        node: dir::GlobalNodeIdAny,
    },
    /// One variable was solved.
    Solve {
        /// The solved variable.
        variable: dir::TypeVariableId,
        /// The solution type.
        solution: dir::GlobalTypeId,
    },
    /// Two open variables were aliased.
    Alias {
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

        if cfg!(debug_assertions) {
            eprintln!("check.event[{}] {event:?}", self.events.len());
        }

        self.events.push(event);
    }

    /// Return rendered event rows for this component.
    pub(in crate::check) fn events(&self) -> ArtifactEventLog {
        let mut log = ArtifactEventLog::new();

        // summarize retained trace state
        log.push(
            ArtifactEvent::new("trace.summary")
                .info()
                .usize("events", self.events.len()),
        );

        // render retained events in order
        for (index, event) in self.events.iter().enumerate() {
            log.push(
                ArtifactEvent::new("check.events")
                    .info()
                    .usize("index", index)
                    .text("event", format!("{event:?}")),
            );
        }

        log
    }

    /// Return derived size counters for this component.
    pub(in crate::check) fn stats(&self) -> CheckStats {
        let mut bounds = 0;
        let mut solutions = 0;
        for (_, state) in self.variables.iter() {
            bounds += state.lower.len() + state.upper.len();
            solutions += usize::from(state.solution.is_some());
        }
        let types = self
            .modules
            .values()
            .map(|module| module.working.types.iter_type_ids().count())
            .sum();

        CheckStats {
            variables: self.variables.count(),
            constraints: self.constraints.count(),
            obligations: self.obligations.count(),
            types,
            solutions,
            bounds,
            decisions: self.decisions.count(),
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
