use destack_artifact::{ArtifactEvent, ArtifactEventLog};
use destack_dir as dir;
use smallvec::SmallVec;

use crate::check::{
    CandidateVerdict, CheckState, ConstraintId, DumpContext, ObligationId, TypeBound, Widening,
};

/// Environment variable naming the file check events stream into.
const CHECK_EVENT_STREAM_ENV: &str = "DESTACK_CHECK_EVENT_STREAM";

/// Derived size counters for one checked module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) struct CheckStats {
    /// The number of allocated variables.
    pub(in crate::check) variables: usize,
    /// The number of collected constraints.
    pub(in crate::check) constraints: usize,
    /// The number of collected obligations.
    pub(in crate::check) obligations: usize,
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
    pub(in crate::check) lower: SmallVec<[TypeBound; 2]>,
    /// Types the variable must be assignable to.
    pub(in crate::check) upper: SmallVec<[TypeBound; 2]>,
}

/// One event emitted by check.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::check) enum CheckEvent {
    /// One speculative probe started.
    ProbeStarted {
        /// The number of variables present before the probe.
        variables: usize,
    },
    /// One speculative probe finished.
    ProbeFinished {
        /// The winnowed verdict, absent when the probe produced none.
        verdict: Option<CandidateVerdict>,
    },
    /// One variable was allocated.
    VariableAllocated {
        /// The allocated variable.
        variable: dir::TypeVariableId,
        /// The literal widening policy applied when solving.
        widening: Widening,
    },
    /// One lower bound was pushed onto an inference variable.
    LowerBoundPushed {
        /// The bounded variable.
        variable: dir::TypeVariableId,
        /// The pushed bound.
        bound: TypeBound,
    },
    /// One upper bound was pushed onto an inference variable.
    UpperBoundPushed {
        /// The bounded variable.
        variable: dir::TypeVariableId,
        /// The pushed bound.
        bound: TypeBound,
    },
    /// One relation constraint was checked.
    RelationChecked {
        /// The constraint.
        constraint: ConstraintId,
        /// Whether the constraint finished.
        is_finished: bool,
    },
    /// One obligation was checked.
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
        bounds: Box<VariableBounds>,
        /// The solution type.
        solution: dir::GlobalTypeId,
    },
}

/// Retained trace state while checking.
pub(in crate::check) struct CheckTrace {
    /// Trace events recorded while checking.
    pub(in crate::check) events: Vec<CheckEvent>,
    /// Whether events are kept for artifact output.
    pub(in crate::check) emit: bool,
    /// Whether events print as they are recorded.
    pub(in crate::check) stream: bool,
}

impl CheckTrace {
    /// Create trace state when emitting or streaming is requested.
    pub(in crate::check) fn new(emit: bool, stream: bool) -> Option<Box<Self>> {
        if !emit && !stream {
            return None;
        }

        Some(Box::new(Self {
            events: Vec::new(),
            emit,
            stream,
        }))
    }
}

impl CheckState<'_> {
    /// Return the recorded trace events, empty without a trace.
    pub(in crate::check) fn trace_events(&self) -> &[CheckEvent] {
        match &self.trace {
            Some(trace) => &trace.events,
            None => &[],
        }
    }

    /// Record one check event.
    pub(in crate::check) fn record_event(&mut self, event: CheckEvent) {
        let Some(trace) = &self.trace else {
            return;
        };

        if trace.stream {
            self.stream_event(&event);
        }
        if let Some(trace) = &mut self.trace
            && trace.emit
        {
            trace.events.push(event);
        }
    }

    /// Print one check event immediately.
    fn stream_event(&self, event: &CheckEvent) {
        let context = DumpContext::new(self);
        let mut log = ArtifactEventLog::new();
        event.render(&context, &mut log);

        eprint!("{}", log.render_plain());
    }

    /// Return rendered event lines for this module.
    pub(in crate::check) fn events(&self) -> ArtifactEventLog {
        let context = DumpContext::new(self);
        let mut log = ArtifactEventLog::new();

        // summarize retained trace state
        log.push(
            ArtifactEvent::new("trace.summary")
                .info()
                .usize("events", self.trace_events().len()),
        );

        // render retained events in order
        for event in self.trace_events() {
            event.render(&context, &mut log);
        }

        log
    }

    /// Return derived size counters for this module.
    pub(in crate::check) fn stats(&self) -> CheckStats {
        let bounds = self.infer.variables.bound_count();
        let mut solutions = 0;
        for (_, state) in self.infer.variables() {
            solutions += usize::from(!state.state.is_open());
        }
        CheckStats {
            variables: self.infer.variable_count(),
            constraints: self.infer.constraint_count(),
            obligations: self.infer.obligation_count(),
            solutions,
            bounds,
            decisions: self.module.decisions.decision_entries().count(),
        }
    }
}

/// Return whether check events should stream as they are recorded.
pub(in crate::check) fn should_stream_check_events() -> bool {
    std::env::var_os(CHECK_EVENT_STREAM_ENV).is_some_and(|value| !value.is_empty() && value != "0")
}

impl CheckStats {
    /// Render these stats as stable metadata lines.
    pub(in crate::check) fn render_metadata(self) -> String {
        format!(
            "\
check.stats.solve.variables={}
check.stats.solve.constraints={}
check.stats.solve.obligations={}
check.stats.solve.solutions={}
check.stats.solve.bounds={}
check.stats.solve.decisions={}",
            self.variables,
            self.constraints,
            self.obligations,
            self.solutions,
            self.bounds,
            self.decisions,
        )
    }
}
