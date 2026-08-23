use destack_artifact::{ArtifactEvent, ArtifactEventLog};
use destack_dir as dir;
use smallvec::SmallVec;

use crate::sema::{Check, CheckId, CheckState, DumpContext, TypeBound, VariableKind, Verdict};

/// Environment variable naming the file check events stream into.
const CHECK_EVENT_STREAM_ENV: &str = "DESTACK_CHECK_EVENT_STREAM";

/// Work counters accumulated while checking one module.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::sema) struct CheckCounters {
    /// Relation judgments decided past the memos.
    pub(in crate::sema) relation_decisions: u64,
    /// Relation judgments served from the memos.
    pub(in crate::sema) relation_reuses: u64,
    /// Member bindings derived past the memo.
    pub(in crate::sema) binding_derivations: u64,
    /// Member bindings served from the memo.
    pub(in crate::sema) binding_reuses: u64,
    /// Speculative probes opened.
    pub(in crate::sema) probes: u64,
    /// Probes opened by overload selection.
    pub(in crate::sema) selection_probes: u64,
    /// Probes opened by extension implementation matching.
    pub(in crate::sema) extension_probes: u64,
    /// Types interned into the working segment.
    pub(in crate::sema) interns: u64,
    /// Type heads reduced.
    pub(in crate::sema) reduces: u64,
    /// Generic parameter instantiations opened.
    pub(in crate::sema) instantiations: u64,
    /// Member lookups served from the answers table.
    pub(in crate::sema) member_reuses: u64,
    /// Member lookups derived past the answers table.
    pub(in crate::sema) member_derivations: u64,
    /// Member lookups refusing canonical form.
    pub(in crate::sema) member_refusals: u64,
    /// Member collections closed coinductively.
    pub(in crate::sema) extension_reentries: u64,
}

/// Derived size counters for one checked module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) struct CheckStats {
    /// The number of allocated variables.
    pub(in crate::sema) variables: usize,
    /// The number of collected constraints.
    pub(in crate::sema) constraints: usize,
    /// The number of collected obligations.
    pub(in crate::sema) obligations: usize,
    /// The number of solved variables.
    pub(in crate::sema) solutions: usize,
    /// The total number of bounds.
    pub(in crate::sema) bounds: usize,
    /// The number of node decisions.
    pub(in crate::sema) decisions: usize,
}

/// The bounds visible when one variable event was traced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::sema) struct VariableBounds {
    /// Types that must be assignable to the variable.
    pub(in crate::sema) lower: SmallVec<[TypeBound; 2]>,
    /// Types the variable must be assignable to.
    pub(in crate::sema) upper: SmallVec<[TypeBound; 2]>,
}

/// One event emitted by check.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::sema) enum CheckEvent {
    /// One speculative probe started.
    ProbeStarted {
        /// The number of variables present before the probe.
        variables: usize,
    },
    /// One speculative probe finished.
    ProbeFinished {
        /// The winnowed verdict, absent when the probe produced none.
        verdict: Option<Verdict>,
    },
    /// One variable was allocated.
    VariableAllocated {
        /// The allocated variable.
        variable: dir::TypeVariableId,
        /// What the variable ranges over.
        kind: VariableKind,
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
    /// One check was stepped.
    Checked {
        /// The stepped check.
        check: CheckId,
        /// Whether the check finished.
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

/// Trace state kept while checking.
pub(in crate::sema) struct CheckTrace {
    /// Trace events kept while checking.
    pub(in crate::sema) events: Vec<CheckEvent>,
    /// Whether events are kept for artifact output.
    pub(in crate::sema) emit: bool,
    /// Whether events print as they arrive.
    pub(in crate::sema) stream: bool,
}

impl CheckTrace {
    /// Create trace state when emitting or streaming is requested.
    pub(in crate::sema) fn new(emit: bool, stream: bool) -> Option<Box<Self>> {
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
    /// Return the kept trace events, empty without a trace.
    pub(in crate::sema) fn trace_events(&self) -> &[CheckEvent] {
        match &self.trace {
            Some(trace) => &trace.events,
            None => &[],
        }
    }

    /// Push one check event onto the trace.
    pub(in crate::sema) fn push_event(&mut self, event: CheckEvent) {
        let Some(trace) = &self.trace else {
            return;
        };

        // print the event as it happens
        if trace.stream {
            self.stream_event(&event);
        }

        // keep the event for artifact output
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
        event.format_event(&context, &mut log);

        eprint!("{}", log.render_plain());
    }

    /// Return the formatted event lines for this module.
    pub(in crate::sema) fn events(&self) -> ArtifactEventLog {
        let context = DumpContext::new(self);
        let mut log = ArtifactEventLog::new();

        // summarize the kept trace state
        log.push(
            ArtifactEvent::new("trace.summary")
                .info()
                .usize("events", self.trace_events().len()),
        );

        // format the kept events in order
        for event in self.trace_events() {
            event.format_event(&context, &mut log);
        }

        log
    }

    /// Return derived size counters for this module.
    pub(in crate::sema) fn stats(&self) -> CheckStats {
        // count the variables that already reached a solution
        let bounds = self.infer.variables.bound_count();
        let mut solutions = 0;
        for (_, state) in self.infer.variables() {
            solutions += usize::from(!state.state.is_open());
        }

        // count relation and declared checks as the constraint and obligation totals
        let mut constraints = 0;
        let mut obligations = 0;
        for (_, check) in self.fulfill.checks.iter() {
            match check {
                Check::Relation(_) => constraints += 1,
                Check::Declared(_) => obligations += 1,
                Check::Node(_)
                | Check::Conversion(_)
                | Check::Narrowing(_)
                | Check::Selection(_)
                | Check::Equality(_) => {}
            }
        }

        CheckStats {
            variables: self.infer.variable_count(),
            constraints,
            obligations,
            solutions,
            bounds,
            decisions: self.module.decisions.decision_entries().count(),
        }
    }
}

/// Return whether check events stream as they arrive.
pub(in crate::sema) fn is_check_event_streaming() -> bool {
    std::env::var_os(CHECK_EVENT_STREAM_ENV).is_some_and(|value| !value.is_empty() && value != "0")
}

impl CheckStats {
    /// Format these stats with their counters as stable metadata lines.
    pub(in crate::sema) fn format_metadata(self, counters: CheckCounters) -> String {
        format!(
            "\
check.stats.solve.variables={}
check.stats.solve.constraints={}
check.stats.solve.obligations={}
check.stats.solve.solutions={}
check.stats.solve.bounds={}
check.stats.solve.decisions={}
check.stats.relations.decided={}
check.stats.relations.reused={}
check.stats.bindings.built={}
check.stats.bindings.reused={}
check.stats.members.derived={}
check.stats.members.reused={}
check.stats.members.refused={}
check.stats.probes.total={}
check.stats.probes.selections={}
check.stats.probes.extensions={}
check.stats.instantiations={}
check.stats.interns={}
check.stats.reduces={}",
            self.variables,
            self.constraints,
            self.obligations,
            self.solutions,
            self.bounds,
            self.decisions,
            counters.relation_decisions,
            counters.relation_reuses,
            counters.binding_derivations,
            counters.binding_reuses,
            counters.member_derivations,
            counters.member_reuses,
            counters.member_refusals,
            counters.probes,
            counters.selection_probes,
            counters.extension_probes,
            counters.instantiations,
            counters.interns,
            counters.reduces,
        )
    }
}
