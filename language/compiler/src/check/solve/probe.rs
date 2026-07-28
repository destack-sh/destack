use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_source::ModuleId;
use smallvec::SmallVec;

use crate::check::{Answer, BodyState, CheckEvent, CheckState, Dependency, SolverSnapshot};
use crate::{CompilerError, CompilerResult};

/// Result of one speculative candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::check) enum CandidateOutcome<T, R> {
    /// The candidate applies.
    Accepted(T),
    /// The candidate does not apply.
    Rejected(R),
}

/// Result of running one candidate attempt to quiescence.
enum CandidateAttempt<T, R> {
    /// The attempt produced an outcome.
    Outcome(Answer<CandidateOutcome<T, R>>),
    /// One drained constraint failed, rejecting the candidate.
    Failed,
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
    reduced_heads: usize,
    /// The reduced type graph count before the probe.
    reduced_graphs: usize,
    /// The decision count before the probe.
    decisions: usize,
    /// The event count before the probe.
    events: usize,
    /// The failed check count before the probe.
    failures: usize,
    /// Module marks before the probe.
    modules: FxIndexMap<ModuleId, ModuleProbeMark>,
}

/// Per-module check state mark before one probe.
#[derive(Debug)]
struct ModuleProbeMark {
    /// Type segment mark before the probe.
    types: dir::TypeMark,
    /// Resolution segment mark before the probe.
    resolutions: dir::ResolutionMark,
    /// Coercion segment mark before the probe.
    coercions: dir::CoercionMark,
    /// Diagnostic count before the probe.
    diagnostics: usize,
    /// Warning count before the probe.
    warnings: usize,
}

impl BodyState<'_, '_> {
    /// Probe one candidate under a rollback, returning its verdict.
    pub(in crate::check) fn probe_candidate<T, R>(
        &mut self,
        mut attempt: impl FnMut(&mut Self) -> CompilerResult<Answer<CandidateOutcome<T, R>>>,
    ) -> CompilerResult<Answer<CandidateVerdict>> {
        let mark = self.check.open_probe();
        let outcome = match self.attempt_candidate(&mark, &mut attempt) {
            Ok(CandidateAttempt::Outcome(outcome)) => Ok(outcome),
            Ok(CandidateAttempt::Failed) => {
                return self.check.reject_probe(mark);
            }
            Err(error) => Err(error),
        };

        self.check.settle_probe(mark, outcome)
    }

    /// Run one candidate attempt, settling its evidence before deciding.
    ///
    /// A pending attempt may only await variables the candidate owns or
    /// touched, so one drain to quiescence either completes it or proves
    /// the candidate undecidable.
    fn attempt_candidate<T, R>(
        &mut self,
        mark: &ProbeMark,
        attempt: &mut impl FnMut(&mut Self) -> CompilerResult<Answer<CandidateOutcome<T, R>>>,
    ) -> CompilerResult<CandidateAttempt<T, R>> {
        let scope = mark.solver.inference_scope();
        loop {
            let outcome = attempt(self)?;
            let Answer::Pending(blockers) = outcome else {
                return Ok(CandidateAttempt::Outcome(outcome));
            };

            // drain every inference layer exposed by this attempt
            let drained = self.check.drain_check_tasks(scope)?;
            let has_failure = self
                .check
                .solver
                .constraints
                .failures_from(mark.solver.constraint_count())
                .next()
                .is_some()
                || self.check.failures.len() > mark.failures;
            if has_failure {
                return Ok(CandidateAttempt::Failed);
            }
            if let Answer::Pending(blockers) = drained {
                return Ok(CandidateAttempt::Outcome(Answer::Pending(blockers)));
            }

            // retry only after a reported dependency completed
            let mut progressed = false;
            for blocker in &blockers {
                progressed |= !self.check.is_dependency_pending(*blocker)?;
            }
            if !progressed {
                return Ok(CandidateAttempt::Outcome(Answer::Pending(blockers)));
            }
        }
    }

    /// Probe one candidate, describing a rejection before the rollback.
    pub(in crate::check) fn probe_candidate_noted<T, R>(
        &mut self,
        mut attempt: impl FnMut(&mut Self) -> CompilerResult<Answer<CandidateOutcome<T, R>>>,
        describe: impl FnOnce(&mut Self, &R) -> CompilerResult<String>,
    ) -> CompilerResult<Answer<(CandidateVerdict, Option<String>)>> {
        let mark = self.check.open_probe();
        let outcome = match self.attempt_candidate(&mark, &mut attempt) {
            Ok(CandidateAttempt::Outcome(outcome)) => Ok(outcome),
            Ok(CandidateAttempt::Failed) => {
                let verdict = self.check.reject_probe(mark)?;

                return Ok(match verdict {
                    Answer::Ready(verdict) => Answer::Ready((verdict, None)),
                    Answer::Pending(blockers) => Answer::Pending(blockers),
                });
            }
            Err(error) => Err(error),
        };

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

    /// Confirm one candidate inside the current transaction.
    pub(in crate::check) fn confirm_candidate<T, R>(
        &mut self,
        mut attempt: impl FnMut(&mut Self) -> CompilerResult<Answer<CandidateOutcome<T, R>>>,
    ) -> CompilerResult<Answer<Option<T>>> {
        let mark = self.check.open_probe();
        let outcome = match self.attempt_candidate(&mark, &mut attempt) {
            Ok(CandidateAttempt::Outcome(outcome)) => outcome,
            Ok(CandidateAttempt::Failed) => {
                self.check.reject_probe(mark)?;

                return Ok(Answer::Ready(None));
            }
            Err(error) => {
                self.check.end_probe(mark)?;

                return Err(error);
            }
        };

        self.check.confirm_probe(mark, outcome)
    }
}

impl CheckState<'_> {
    /// Return whether one dependency still awaits its value.
    pub(in crate::check) fn is_dependency_pending(
        &self,
        dependency: Dependency,
    ) -> CompilerResult<bool> {
        let is_pending = match dependency {
            Dependency::Variable(variable) => self.solver.variable(variable)?.state.is_open(),
            Dependency::SymbolType(symbol) => self.symbol_type_maybe(symbol).is_none(),
            Dependency::NodeType(node) => !self.node_types.contains_key(&node),
        };

        Ok(is_pending)
    }

    /// Probe one candidate under a rollback, returning its verdict.
    pub(in crate::check) fn probe_candidate<T, R>(
        &mut self,
        mut attempt: impl FnMut(&mut Self) -> CompilerResult<Answer<CandidateOutcome<T, R>>>,
    ) -> CompilerResult<Answer<CandidateVerdict>> {
        let mark = self.open_probe();
        let outcome = attempt(self);

        self.settle_probe(mark, outcome)
    }

    /// Confirm one candidate inside the current transaction.
    pub(in crate::check) fn confirm_candidate<T, R>(
        &mut self,
        mut attempt: impl FnMut(&mut Self) -> CompilerResult<Answer<CandidateOutcome<T, R>>>,
    ) -> CompilerResult<Answer<Option<T>>> {
        let mark = self.open_probe();
        let outcome = attempt(self);
        let outcome = match outcome {
            Ok(outcome) => outcome,
            Err(error) => {
                self.end_probe(mark)?;

                return Err(error);
            }
        };

        self.confirm_probe(mark, outcome)
    }

    /// Commit one accepted candidate or roll its transaction back.
    fn confirm_probe<T, R>(
        &mut self,
        mark: ProbeMark,
        outcome: Answer<CandidateOutcome<T, R>>,
    ) -> CompilerResult<Answer<Option<T>>> {
        let verdict = match &outcome {
            Answer::Ready(CandidateOutcome::Accepted(_)) => self.settle_probe_tasks(&mark),
            Answer::Ready(CandidateOutcome::Rejected(_)) => {
                Ok(Answer::Ready(CandidateVerdict::Rejected))
            }
            Answer::Pending(blockers) => self.classify_probe_blockers(&mark, blockers.clone()),
        };
        let verdict = match verdict {
            Ok(verdict) => verdict,
            Err(error) => {
                self.end_probe(mark)?;

                return Err(error);
            }
        };

        // commit complete candidates and provisional children
        match verdict {
            Answer::Ready(CandidateVerdict::Viable) => {
                let Answer::Ready(CandidateOutcome::Accepted(selected)) = outcome else {
                    self.end_probe(mark)?;

                    return Err(CompilerError::Internal {
                        message: "viable probe has no accepted outcome".into(),
                    });
                };
                self.record_event(CheckEvent::ProbeFinished {
                    verdict: Some(CandidateVerdict::Viable),
                });
                self.commit_probe(mark)?;

                Ok(Answer::Ready(Some(selected)))
            }
            Answer::Ready(CandidateVerdict::Rejected) => {
                self.reject_probe(mark)?;

                Ok(Answer::Ready(None))
            }
            Answer::Ready(CandidateVerdict::Indeterminate) => {
                let Answer::Ready(CandidateOutcome::Accepted(selected)) = outcome else {
                    self.end_probe(mark)?;

                    return Err(CompilerError::Internal {
                        message: "indeterminate confirmed candidate has no accepted outcome".into(),
                    });
                };
                self.record_event(CheckEvent::ProbeFinished {
                    verdict: Some(CandidateVerdict::Indeterminate),
                });
                self.commit_probe(mark)?;

                Ok(Answer::Ready(Some(selected)))
            }
            Answer::Pending(blockers) => {
                self.end_probe(mark)?;

                Ok(Answer::Pending(blockers))
            }
        }
    }

    /// Close one probe whose drained constraints failed.
    fn reject_probe(&mut self, mark: ProbeMark) -> CompilerResult<Answer<CandidateVerdict>> {
        self.record_event(CheckEvent::ProbeFinished {
            verdict: Some(CandidateVerdict::Rejected),
        });
        self.end_probe(mark)?;

        Ok(Answer::Ready(CandidateVerdict::Rejected))
    }

    /// Begin one probe, recording its start event.
    fn open_probe(&mut self) -> ProbeMark {
        let mark = self.begin_probe();
        self.record_event(CheckEvent::ProbeStarted {
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
            Ok(Answer::Ready(CandidateOutcome::Accepted(_))) => self.settle_probe_tasks(&mark),
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

    /// Settle checks produced by one accepted candidate.
    fn settle_probe_tasks(&mut self, mark: &ProbeMark) -> CompilerResult<Answer<CandidateVerdict>> {
        // the candidate settles what it owns and what it adopted
        let scope = mark.solver.inference_scope();
        let drained = self.drain_check_tasks(scope)?;
        let has_failure = self
            .solver
            .constraints
            .failures_from(mark.solver.constraint_count())
            .next()
            .is_some()
            || self.failures.len() > mark.failures;
        if has_failure {
            return Ok(Answer::Ready(CandidateVerdict::Rejected));
        }
        if let Answer::Pending(blockers) = drained {
            return self.classify_probe_blockers(mark, blockers);
        }

        // unresolved candidate variables keep the candidate indeterminate
        if !self.open_variables(scope)?.is_empty() {
            return Ok(Answer::Ready(CandidateVerdict::Indeterminate));
        }

        let blockers = self.solver.waiting_dependencies();
        if blockers.is_empty() {
            return Ok(Answer::Ready(CandidateVerdict::Viable));
        }

        self.classify_probe_blockers(mark, blockers.into())
    }

    /// Classify dependencies returned directly by one candidate attempt.
    fn classify_probe_blockers(
        &self,
        mark: &ProbeMark,
        blockers: SmallVec<[Dependency; 2]>,
    ) -> CompilerResult<Answer<CandidateVerdict>> {
        let mut external = SmallVec::new();
        for dependency in blockers
            .into_iter()
            .chain(self.solver.waiting_dependencies())
        {
            match dependency {
                // discard inference owned by the rolled back candidate
                Dependency::Variable(variable) if !mark.solver.contains_variable(variable) => {}

                // retain dependencies owned by the enclosing check
                Dependency::Variable(_) | Dependency::SymbolType(_) | Dependency::NodeType(_) => {
                    if !external.contains(&dependency) {
                        external.push(dependency);
                    }
                }
            }
        }
        if !external.is_empty() {
            return Ok(Answer::Pending(external));
        }

        Ok(Answer::Ready(CandidateVerdict::Indeterminate))
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
                        coercions: state.coercions.mark(),
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
            reduced_heads: self.reduced_heads.len(),
            reduced_graphs: self.reduced_graphs.len(),
            decisions: self.decisions.count(),
            events: self.events.len(),
            failures: self.failures.len(),
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
            reduced_heads,
            reduced_graphs,
            decisions,
            events,
            failures,
            modules,
        } = mark;

        self.solver.rollback(solver)?;
        self.drop_probe_state(
            node_types,
            declaration_types,
            binding_types,
            reduced_heads,
            reduced_graphs,
            decisions,
            events,
            failures,
            modules,
        );

        Ok(())
    }

    /// Commit one successful probe.
    fn commit_probe(&mut self, mark: ProbeMark) -> CompilerResult<()> {
        self.solver.commit(mark.solver);

        // retry outer work whose dependency completed inside the probe
        for dependency in self.solver.waiting_dependencies() {
            if self.is_dependency_pending(dependency)? {
                continue;
            }
            let waiters = self.solver.wake(dependency);
            for waiter in waiters {
                self.queue_task(waiter);
            }
        }

        Ok(())
    }

    /// Drop state allocated inside a rolled back probe.
    fn drop_probe_state(
        &mut self,
        node_types: usize,
        declaration_types: usize,
        binding_types: usize,
        reduced_heads: usize,
        reduced_graphs: usize,
        decisions: usize,
        events: usize,
        failures: usize,
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
        while self.reduced_heads.len() > reduced_heads {
            self.reduced_heads.pop();
        }
        while self.reduced_graphs.len() > reduced_graphs {
            self.reduced_graphs.pop();
        }
        self.decisions.truncate_to(decisions);
        self.events.truncate(events);
        self.failures.truncate(failures);

        for (module, mark) in modules {
            if let Some(state) = self.modules.get_mut(&module) {
                state.types_tail.truncate_to(mark.types);
                state.resolutions.truncate_to(mark.resolutions);
                state.coercions.truncate_to(mark.coercions);
                state.diagnostics.truncate(mark.diagnostics);
                state.warnings.truncate(mark.warnings);
            }
        }
    }
}
