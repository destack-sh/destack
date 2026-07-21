use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::CompilerResult;
use crate::check::{Answer, BodyState, CheckEvent, CheckState, Dependency, SolverSnapshot};

/// Result of one speculative candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum CandidateOutcome<T, R> {
    /// The candidate applies.
    Accepted(T),
    /// The candidate does not apply.
    Rejected(R),
}

impl<T, R> Answer<CandidateOutcome<T, R>> {
    /// Return the accepted value carried by one confirmed candidate.
    fn accepted(self) -> Answer<Option<T>> {
        match self {
            Self::Ready(CandidateOutcome::Accepted(value)) => Answer::Ready(Some(value)),
            Self::Ready(CandidateOutcome::Rejected(_)) => Answer::Ready(None),
            Self::Pending(blockers) => Answer::Pending(blockers),
        }
    }
}

/// Verdict of one winnowed candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum CandidateVerdict {
    /// The candidate applies.
    Viable,
    /// Probe-local inference cannot decide whether the candidate applies.
    Indeterminate,
    /// The candidate does not apply.
    Rejected,
}

/// The judgment one probe speculates on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum ProbeReason {
    /// One signature candidate against a call.
    Signature,
    /// One extension target against a receiver.
    Extension,
    /// One extension's interface declarations against a receiver.
    Implements,
    /// One extension against a protocol member set.
    Protocol,
    /// One receiver adjustment step.
    Receiver,
    /// One conditional type pattern match.
    Conditional,
    /// One union arm against a related value.
    UnionArm,
}

/// Check state mark before one probe.
#[derive(Debug)]
pub(in crate::check) struct ProbeMark {
    /// The solver state before the probe.
    solver: SolverSnapshot,
    /// The node type count before the probe.
    node_types: usize,
    /// The declaration type count before the probe.
    declaration_types: usize,
    /// The binding type count before the probe.
    binding_types: usize,
    /// The reduced type count before the probe.
    reduced_types: usize,
    /// The reduced type graph count before the probe.
    reduced_type_graphs: usize,
    /// The contextual place count before the probe.
    contextual_places: usize,
    /// The decision count before the probe.
    decisions: usize,
    /// The event count before the probe.
    events: usize,
    /// Module marks before the probe.
    modules: FxIndexMap<ModuleId, ModuleProbeMark>,
}

/// Per-module check state mark before one probe.
#[derive(Debug)]
struct ModuleProbeMark {
    /// Working type segment mark before the probe.
    types: dir::TypeMark,
    /// Resolution segment mark before the probe.
    resolutions: dir::ResolutionMark,
    /// Diagnostic count before the probe.
    diagnostics: usize,
    /// Warning count before the probe.
    warnings: usize,
}

impl BodyState<'_, '_> {
    /// Probe one candidate under a rollback, returning its verdict.
    pub(in crate::check) fn probe_candidate<T, R>(
        &mut self,
        reason: ProbeReason,
        mut attempt: impl FnMut(&mut Self) -> CompilerResult<Answer<CandidateOutcome<T, R>>>,
    ) -> CompilerResult<Answer<CandidateVerdict>> {
        let mark = self.check.open_probe(reason);
        let outcome = attempt(self);

        self.check.settle_probe(mark, outcome)
    }

    /// Probe one candidate, describing a rejection before the rollback.
    pub(in crate::check) fn probe_candidate_noted<T, R>(
        &mut self,
        reason: ProbeReason,
        mut attempt: impl FnMut(&mut Self) -> CompilerResult<Answer<CandidateOutcome<T, R>>>,
        describe: impl FnOnce(&mut Self, &R) -> CompilerResult<String>,
    ) -> CompilerResult<Answer<(CandidateVerdict, Option<String>)>> {
        let mark = self.check.open_probe(reason);
        let outcome = attempt(self);

        // rejection payloads reference probe types, so describe them
        //  before the rollback frees their interned slots
        let note = match &outcome {
            Ok(Answer::Ready(CandidateOutcome::Rejected(rejection))) => {
                Some(describe(self, rejection)?)
            }
            _ => None,
        };
        let verdict = self.check.settle_probe(mark, outcome)?;

        Ok(match verdict {
            Answer::Ready(verdict) => Answer::Ready((verdict, note)),
            Answer::Pending(blockers) => Answer::Pending(blockers),
        })
    }

    /// Probe one candidate, then confirm a viable outcome in place.
    pub(in crate::check) fn confirm_candidate<T, R>(
        &mut self,
        reason: ProbeReason,
        mut attempt: impl FnMut(&mut Self) -> CompilerResult<Answer<CandidateOutcome<T, R>>>,
    ) -> CompilerResult<Answer<Option<T>>> {
        match self.probe_candidate(reason, &mut attempt)? {
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
            Answer::Ready(CandidateVerdict::Rejected) => Ok(Answer::Ready(None)),
            Answer::Ready(CandidateVerdict::Viable | CandidateVerdict::Indeterminate) => {
                Ok(attempt(self)?.accepted())
            }
        }
    }
}

impl CheckState<'_> {
    /// Probe one candidate under a rollback, returning its verdict.
    pub(in crate::check) fn probe_candidate<T, R>(
        &mut self,
        reason: ProbeReason,
        mut attempt: impl FnMut(&mut Self) -> CompilerResult<Answer<CandidateOutcome<T, R>>>,
    ) -> CompilerResult<Answer<CandidateVerdict>> {
        let mark = self.open_probe(reason);
        let outcome = attempt(self);

        self.settle_probe(mark, outcome)
    }

    /// Probe one candidate, then confirm a viable outcome in place.
    pub(in crate::check) fn confirm_candidate<T, R>(
        &mut self,
        reason: ProbeReason,
        mut attempt: impl FnMut(&mut Self) -> CompilerResult<Answer<CandidateOutcome<T, R>>>,
    ) -> CompilerResult<Answer<Option<T>>> {
        match self.probe_candidate(reason, &mut attempt)? {
            Answer::Pending(blockers) => Ok(Answer::Pending(blockers)),
            Answer::Ready(CandidateVerdict::Rejected) => Ok(Answer::Ready(None)),
            Answer::Ready(CandidateVerdict::Viable | CandidateVerdict::Indeterminate) => {
                Ok(attempt(self)?.accepted())
            }
        }
    }

    /// Begin one probe, recording its start event.
    fn open_probe(&mut self, reason: ProbeReason) -> ProbeMark {
        let mark = self.begin_probe();
        self.record_event(CheckEvent::ProbeStarted {
            reason,
            variables: self.solver.variable_count(),
        });

        mark
    }

    /// Map one attempted outcome onto its verdict, rolling the probe back.
    fn settle_probe<T, R>(
        &mut self,
        mark: ProbeMark,
        outcome: CompilerResult<Answer<CandidateOutcome<T, R>>>,
    ) -> CompilerResult<Answer<CandidateVerdict>> {
        let verdict = match outcome {
            Ok(Answer::Ready(CandidateOutcome::Accepted(_))) => {
                self.settle_probe_constraints(&mark)
            }
            Ok(Answer::Ready(CandidateOutcome::Rejected(_))) => {
                Ok(Answer::Ready(CandidateVerdict::Rejected))
            }
            Ok(Answer::Pending(blockers)) => self.classify_probe_blockers(&mark, blockers),
            Err(error) => {
                self.end_probe(mark)?;

                return Err(error);
            }
        };
        let verdict = match verdict {
            Ok(verdict) => verdict,
            Err(error) => {
                self.end_probe(mark)?;

                return Err(error);
            }
        };

        self.record_event(CheckEvent::ProbeFinished {
            verdict: verdict.ready_ref().copied(),
        });
        self.end_probe(mark)?;

        Ok(verdict)
    }

    /// Settle type constraints produced by one accepted candidate.
    fn settle_probe_constraints(
        &mut self,
        mark: &ProbeMark,
    ) -> CompilerResult<Answer<CandidateVerdict>> {
        let variables = mark.solver.variable_domain();
        let failures = self.drain_constraint_tasks(variables)?;
        if !failures.is_empty() {
            return Ok(Answer::Ready(CandidateVerdict::Rejected));
        }

        let blockers = self.solver.waiting_dependencies();
        if blockers.is_empty() {
            return Ok(Answer::Ready(CandidateVerdict::Viable));
        }

        self.classify_probe_blockers(mark, blockers.into())
    }

    /// Classify dependencies returned directly by one candidate judgment.
    fn classify_probe_blockers(
        &self,
        mark: &ProbeMark,
        blockers: SmallVec<[Dependency; 2]>,
    ) -> CompilerResult<Answer<CandidateVerdict>> {
        let local = self.probe_variables(mark, &blockers)?;
        if local.is_empty() {
            Ok(Answer::Pending(blockers))
        } else {
            Ok(Answer::Ready(CandidateVerdict::Indeterminate))
        }
    }

    /// Return unresolved variables allocated by one probe.
    fn probe_variables(
        &self,
        mark: &ProbeMark,
        blockers: &[Dependency],
    ) -> CompilerResult<SmallVec<[dir::TypeVariableId; 2]>> {
        let mut variables = SmallVec::new();

        for blocker in blockers {
            let Dependency::Variable(variable) = blocker else {
                continue;
            };
            let variable = *variable;
            if mark.solver.contains_variable(variable) || self.solver.solution(variable)?.is_some()
            {
                continue;
            }
            if !variables.contains(&variable) {
                variables.push(variable);
            }
        }

        Ok(variables)
    }

    /// Begin one probe.
    fn begin_probe(&mut self) -> ProbeMark {
        let modules = self
            .modules
            .iter()
            .map(|(module, state)| {
                (
                    *module,
                    ModuleProbeMark {
                        types: state.types_tail.mark(),
                        resolutions: state.resolutions.mark(),
                        diagnostics: state.diagnostics.len(),
                        warnings: state.warnings.len(),
                    },
                )
            })
            .collect();
        let solver = self.solver.snapshot();

        ProbeMark {
            solver,
            node_types: self.node_types.len(),
            declaration_types: self.declaration_types.len(),
            binding_types: self.binding_types.len(),
            reduced_types: self.reduced_types.len(),
            reduced_type_graphs: self.reduced_type_graphs.len(),
            contextual_places: self.contextual_places.len(),
            decisions: self.decisions.count(),
            events: self.events.len(),
            modules,
        }
    }

    /// End one probe, rolling its state back.
    fn end_probe(&mut self, mark: ProbeMark) -> CompilerResult<()> {
        let ProbeMark {
            solver,
            node_types,
            declaration_types,
            binding_types,
            reduced_types,
            reduced_type_graphs,
            contextual_places,
            decisions,
            events,
            modules,
        } = mark;

        self.solver.rollback(solver)?;
        self.drop_probe_state(
            node_types,
            declaration_types,
            binding_types,
            reduced_types,
            reduced_type_graphs,
            contextual_places,
            decisions,
            events,
            modules,
        );

        Ok(())
    }

    /// Drop state allocated inside a rolled back probe.
    fn drop_probe_state(
        &mut self,
        node_types: usize,
        declaration_types: usize,
        binding_types: usize,
        reduced_types: usize,
        reduced_type_graphs: usize,
        contextual_places: usize,
        decisions: usize,
        events: usize,
        modules: FxIndexMap<ModuleId, ModuleProbeMark>,
    ) {
        while self.node_types.len() > node_types {
            self.node_types.pop();
        }
        while self.declaration_types.len() > declaration_types {
            self.declaration_types.pop();
        }
        while self.binding_types.len() > binding_types {
            self.binding_types.pop();
        }
        while self.reduced_types.len() > reduced_types {
            self.reduced_types.pop();
        }
        while self.reduced_type_graphs.len() > reduced_type_graphs {
            self.reduced_type_graphs.pop();
        }
        while self.contextual_places.len() > contextual_places {
            self.contextual_places.pop();
        }
        self.decisions.truncate_to(decisions);
        self.events.truncate(events);

        for (module, mark) in modules {
            if let Some(state) = self.modules.get_mut(&module) {
                state.types_tail.truncate_to(mark.types);
                state.resolutions.truncate_to(mark.resolutions);
                state.diagnostics.truncate(mark.diagnostics);
                state.warnings.truncate(mark.warnings);
            }
        }
    }
}
