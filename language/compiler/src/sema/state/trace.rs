use smallvec::SmallVec;
use tspp_dir as dir;
use tspp_repository::{ProviderContext, TraceEvent};

use crate::CompilerResult;
use crate::sema::{Bound, Check, CheckId, CheckState, EventFormatter, VariableKind};

/// Environment variable naming the file check events stream into.
const CHECK_EVENT_STREAM_ENV: &str = "TSPP_CHECK_EVENT_STREAM";

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
    /// Types interned into the working segment.
    pub(in crate::sema) interns: u64,
    /// Type heads reduced.
    pub(in crate::sema) reduces: u64,
    /// Generic parameter instantiations opened.
    pub(in crate::sema) instantiations: u64,
    /// Member lookups derived.
    pub(in crate::sema) member_derivations: u64,
    /// Member lookups refusing canonical form.
    pub(in crate::sema) member_refusals: u64,
    /// Member collections closed coinductively.
    pub(in crate::sema) extension_reentries: u64,
    /// Instances the closure materialized.
    pub(in crate::sema) instances: u64,
    /// Entries materialized under an instance's substitution.
    pub(in crate::sema) instance_entries: u64,
}

/// The bounds visible when one variable event was traced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::sema) struct VariableBounds {
    /// Types that must be assignable to the variable.
    pub(in crate::sema) lower: SmallVec<[Bound; 2]>,
    /// Types the variable must be assignable to.
    pub(in crate::sema) upper: SmallVec<[Bound; 2]>,
}

/// One event recorded by check.
#[derive(Debug)]
pub(in crate::sema) enum CheckEvent {
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
        bound: Bound,
    },
    /// One upper bound was pushed onto an inference variable.
    UpperBoundPushed {
        /// The bounded variable.
        variable: dir::TypeVariableId,
        /// The pushed bound.
        bound: Bound,
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
    /// Events recorded while checking.
    pub(in crate::sema) events: Vec<TraceEvent>,
    /// Whether events are retained by the provider trace.
    pub(in crate::sema) records_events: bool,
    /// Whether events print as they arrive.
    pub(in crate::sema) is_streaming: bool,
}

impl CheckTrace {
    /// Create trace state when recording or streaming is requested.
    pub(in crate::sema) fn new(records_events: bool) -> Option<Box<Self>> {
        let is_streaming = std::env::var_os(CHECK_EVENT_STREAM_ENV)
            .is_some_and(|value| !value.is_empty() && value != "0");
        if !records_events && !is_streaming {
            return None;
        }

        // start an empty trace
        Some(Box::new(Self {
            events: Vec::new(),
            records_events,
            is_streaming,
        }))
    }

    /// Stream and retain one formatted event.
    fn record(&mut self, event: TraceEvent) {
        // print the event as it happens
        if self.is_streaming {
            eprintln!("{event}");
        }

        // retain the event for the provider trace
        if self.records_events {
            self.events.push(event);
        }
    }
}

impl CheckState<'_> {
    /// Record one check event.
    pub(in crate::sema) fn record_event(&mut self, event: CheckEvent) {
        if self.trace.is_none() {
            return;
        }

        // format against the exact state visible when recorded
        let event = EventFormatter::new(self).format(&event);

        if let Some(trace) = &mut self.trace {
            trace.record(event);
        }
    }

    /// Record one check step against its exact retained check.
    pub(in crate::sema) fn record_check_event(
        &mut self,
        id: CheckId,
        is_finished: bool,
    ) -> CompilerResult<()> {
        if self.trace.is_none() {
            return Ok(());
        }

        // format the check before mutating its trace buffer
        let check = self.fulfill.checks.get(id)?.clone();
        let event = EventFormatter::new(self).format_check(&check, id, is_finished);

        // record the event on the live trace
        if let Some(trace) = &mut self.trace {
            trace.record(event);
        }

        Ok(())
    }

    /// Record this check's counters and retained events.
    pub(in crate::sema) fn record_trace(&mut self, context: &dyn ProviderContext) {
        if !context.records_timings() {
            return;
        }

        // count the variables that already reached a solution
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
                Check::Conversion(_) | Check::Body(_) | Check::Pattern(_) | Check::Place(_) => {}
            }
        }

        // record current table sizes and accumulated work
        context.record_counters(&[
            ("check.solve.variables", self.infer.variable_count() as u64),
            ("check.solve.constraints", constraints as u64),
            ("check.solve.obligations", obligations as u64),
            ("check.solve.solutions", solutions as u64),
            (
                "check.solve.decisions",
                self.module.decisions_tail.decision_entries().count() as u64,
            ),
        ]);
        self.counters.record(context);

        // move detailed events into their owning attempt
        if context.records_events()
            && let Some(trace) = &mut self.trace
        {
            context.record_events(std::mem::take(&mut trace.events));
        }
    }
}

impl CheckCounters {
    /// Record these counters in one provider attempt.
    pub(in crate::sema) fn record(self, context: &dyn ProviderContext) {
        context.record_counters(&[
            ("check.relations.decided", self.relation_decisions),
            ("check.relations.reused", self.relation_reuses),
            ("check.bindings.built", self.binding_derivations),
            ("check.bindings.reused", self.binding_reuses),
            ("check.members.derived", self.member_derivations),
            ("check.members.refused", self.member_refusals),
            ("check.instantiations", self.instantiations),
            ("check.interns", self.interns),
            ("check.reduces", self.reduces),
            ("materialize.instances", self.instances),
            ("materialize.instance_entries", self.instance_entries),
        ]);
    }
}
