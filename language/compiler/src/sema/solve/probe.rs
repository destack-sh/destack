use destack_core::FxIndexMap;
use destack_dir as dir;
use destack_source::ModuleId;

use crate::sema::{
    BodyState, CheckEvent, CheckOutcome, CheckState, ConstraintId, PendingWork, Settle, TrailMark,
    WorkState,
};
use crate::{CompilerError, CompilerResult};

/// Result of one speculative candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::sema) enum CandidateOutcome<T, R> {
    /// The candidate applies.
    Accepted(T),
    /// The candidate does not apply.
    Rejected(R),
}

/// Result of running one candidate attempt.
enum CandidateAttempt<T, R> {
    /// The attempt produced an outcome.
    Outcome(CandidateOutcome<T, R>),
    /// One constraint solved during the attempt failed, rejecting the candidate.
    Failed,
}

/// Verdict of one winnowed candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum CandidateVerdict {
    /// The candidate applies.
    Viable,
    /// Probe-local inference cannot decide whether the candidate applies.
    Indeterminate,
    /// The candidate does not apply.
    Rejected,
}

/// Check state mark before one probe.
#[derive(Debug)]
pub(in crate::sema) struct ProbeMark {
    /// The inference trail mark before the probe.
    trail: TrailMark,
    /// The node type count before the probe.
    node_types: usize,
    /// The selection memo count before the probe.
    selections: usize,
    /// The checked function body count before the probe.
    functions: usize,
    /// The checked function value count before the probe.
    lambdas: usize,
    /// The walked declaration count before the probe.
    walked_declarations: usize,
    /// The declaration type count before the probe.
    declaration_types: usize,
    /// The binding type count before the probe.
    binding_types: usize,
    /// The symbol variable count before the probe.
    symbol_variables: usize,
    /// The contextual expected type count before the probe.
    expected_types: usize,
    /// The pending work count before the probe.
    pending: usize,
    /// The event count before the probe.
    events: usize,
    /// The failed check count before the probe.
    failures: usize,
    /// The per-module marks before the probe.
    modules: FxIndexMap<ModuleId, ModuleProbeMark>,
}

/// Per-module check state mark before one probe.
#[derive(Debug)]
struct ModuleProbeMark {
    /// Decision segment mark before the probe.
    decisions: dir::DecisionMark,
    /// Member segment mark before the probe.
    members: dir::MemberMark,
    /// Coercion segment mark before the probe.
    coercions: dir::CoercionMark,
    /// Diagnostic count before the probe.
    diagnostics: usize,
    /// Warning count before the probe.
    warnings: usize,
}

impl BodyState<'_, '_> {
    /// Probe one candidate under a rollback, returning its verdict.
    pub(in crate::sema) fn probe_candidate<T, R>(
        &mut self,
        mut attempt: impl FnMut(&mut Self) -> CompilerResult<CandidateOutcome<T, R>>,
    ) -> CompilerResult<CandidateVerdict> {
        let mark = self.check.open_probe();
        let outcome = self.attempt_candidate(&mark, false, &mut attempt);

        self.check.finish_probe(mark, outcome)
    }

    /// Run one candidate attempt, rejecting it on failed constraints.
    fn attempt_candidate<T, R>(
        &mut self,
        mark: &ProbeMark,
        fulfill: bool,
        attempt: &mut impl FnMut(&mut Self) -> CompilerResult<CandidateOutcome<T, R>>,
    ) -> CompilerResult<CandidateAttempt<T, R>> {
        let outcome = attempt(self)?;

        // solve the attempt's queued constraints before the verdict
        let mut has_failed_constraint = false;
        if fulfill {
            // fulfill the candidate's own scope to a fixpoint
            let scope = mark.trail.inference_scope();
            self.check.fulfill_scope(scope, Settle::Complete)?;

            // re-solve the stalled pending set, not only fresh allocations
            let pending = self
                .check
                .fulfill
                .work
                .iter()
                .filter_map(|work| match (work.state, work.kind) {
                    (WorkState::Done, _) => None,
                    (_, PendingWork::Constraint(id)) => Some(id),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let fresh = mark.trail.constraint_count()..self.check.fulfill.constraint_count();
            for id in pending.into_iter().chain(fresh.map(ConstraintId::at)) {
                self.check.solve_constraint(id, Settle::Complete)?;
                if let Some(result) = self.check.fulfill.constraints.result(id)? {
                    has_failed_constraint |= matches!(result.outcome, CheckOutcome::Fails(_));
                }
            }
        }

        // reject the attempt when a constraint it solved failed
        let has_failure = self
            .check
            .fulfill
            .constraints
            .failures_from(mark.trail.constraint_count())
            .next()
            .is_some()
            || self.check.fulfill.failures.len() > mark.failures;

        if has_failure || has_failed_constraint {
            Ok(CandidateAttempt::Failed)
        } else {
            Ok(CandidateAttempt::Outcome(outcome))
        }
    }

    /// Probe one candidate, describing a rejection before the rollback.
    pub(in crate::sema) fn probe_candidate_describing<T, R>(
        &mut self,
        mut attempt: impl FnMut(&mut Self) -> CompilerResult<CandidateOutcome<T, R>>,
        describe: impl FnOnce(&mut Self, &R) -> CompilerResult<String>,
    ) -> CompilerResult<(CandidateVerdict, Option<String>)> {
        self.check.counters.selection_probes += 1;
        let mark = self.check.open_probe();
        let outcome = self.attempt_candidate(&mark, true, &mut attempt);

        // describe the rejection before the rollback drops its state
        let note = match &outcome {
            Ok(CandidateAttempt::Outcome(CandidateOutcome::Rejected(rejection))) => {
                Some(describe(self, rejection)?)
            }
            _ => None,
        };
        let verdict = self.check.finish_probe(mark, outcome)?;

        Ok((verdict, note))
    }

    /// Confirm one candidate inside the current transaction.
    pub(in crate::sema) fn confirm_candidate<T, R>(
        &mut self,
        mut attempt: impl FnMut(&mut Self) -> CompilerResult<CandidateOutcome<T, R>>,
    ) -> CompilerResult<Option<T>> {
        let mark = self.check.open_probe();
        let outcome = match self.attempt_candidate(&mark, false, &mut attempt) {
            Ok(CandidateAttempt::Outcome(outcome)) => outcome,
            Ok(CandidateAttempt::Failed) => {
                self.check.reject_probe(mark)?;

                return Ok(None);
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
    /// Probe one candidate under a rollback on the plain check state.
    pub(in crate::sema) fn probe_candidate<T, R>(
        &mut self,
        mut attempt: impl FnMut(&mut Self) -> CompilerResult<CandidateOutcome<T, R>>,
    ) -> CompilerResult<CandidateVerdict> {
        let mark = self.open_probe();
        let outcome = attempt(self).map(CandidateAttempt::Outcome);

        self.finish_probe(mark, outcome)
    }

    /// Confirm one candidate inside the current transaction on the plain check state.
    pub(in crate::sema) fn confirm_candidate<T, R>(
        &mut self,
        mut attempt: impl FnMut(&mut Self) -> CompilerResult<CandidateOutcome<T, R>>,
    ) -> CompilerResult<Option<T>> {
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
        outcome: CandidateOutcome<T, R>,
    ) -> CompilerResult<Option<T>> {
        let verdict = match &outcome {
            CandidateOutcome::Accepted(_) => self.accepted_verdict(&mark),
            CandidateOutcome::Rejected(_) => Ok(CandidateVerdict::Rejected),
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
            CandidateVerdict::Viable => {
                let CandidateOutcome::Accepted(selected) = outcome else {
                    self.end_probe(mark)?;

                    return Err(CompilerError::Internal {
                        message: "viable probe has no accepted outcome".into(),
                    });
                };
                self.record_event(CheckEvent::ProbeFinished {
                    verdict: Some(CandidateVerdict::Viable),
                });
                self.commit_probe(mark)?;

                Ok(Some(selected))
            }
            CandidateVerdict::Rejected => {
                self.reject_probe(mark)?;

                Ok(None)
            }
            CandidateVerdict::Indeterminate => {
                let CandidateOutcome::Accepted(selected) = outcome else {
                    self.end_probe(mark)?;

                    return Err(CompilerError::Internal {
                        message: "indeterminate confirmed candidate has no accepted outcome".into(),
                    });
                };
                self.record_event(CheckEvent::ProbeFinished {
                    verdict: Some(CandidateVerdict::Indeterminate),
                });
                self.commit_probe(mark)?;

                Ok(Some(selected))
            }
        }
    }

    /// Close one probe whose solved constraints failed.
    fn reject_probe(&mut self, mark: ProbeMark) -> CompilerResult<CandidateVerdict> {
        self.record_event(CheckEvent::ProbeFinished {
            verdict: Some(CandidateVerdict::Rejected),
        });
        self.end_probe(mark)?;

        Ok(CandidateVerdict::Rejected)
    }

    /// Map one attempted outcome onto its verdict, rolling the probe back.
    fn finish_probe<T, R>(
        &mut self,
        mark: ProbeMark,
        outcome: CompilerResult<CandidateAttempt<T, R>>,
    ) -> CompilerResult<CandidateVerdict> {
        let verdict = match outcome {
            Ok(CandidateAttempt::Outcome(CandidateOutcome::Accepted(_))) => {
                self.accepted_verdict(&mark)
            }
            Ok(CandidateAttempt::Outcome(CandidateOutcome::Rejected(_)))
            | Ok(CandidateAttempt::Failed) => Ok(CandidateVerdict::Rejected),
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
            verdict: Some(verdict),
        });
        self.end_probe(mark)?;

        Ok(verdict)
    }

    /// Return the verdict of one accepted candidate's probe.
    fn accepted_verdict(&mut self, mark: &ProbeMark) -> CompilerResult<CandidateVerdict> {
        let scope = mark.trail.inference_scope();
        let has_failure = self
            .fulfill
            .constraints
            .failures_from(mark.trail.constraint_count())
            .next()
            .is_some()
            || self.fulfill.failures.len() > mark.failures;
        if has_failure {
            return Ok(CandidateVerdict::Rejected);
        }

        // keep the candidate indeterminate while its variables stay open
        if !self.open_scope_variables(scope)?.is_empty() {
            return Ok(CandidateVerdict::Indeterminate);
        }

        Ok(CandidateVerdict::Viable)
    }

    /// Begin one probe, recording its start event.
    fn open_probe(&mut self) -> ProbeMark {
        self.counters.probes += 1;
        self.record_event(CheckEvent::ProbeStarted {
            variables: self.infer.variable_count(),
        });

        let modules = [&self.module]
            .into_iter()
            .map(|state| {
                (
                    self.module_id,
                    ModuleProbeMark {
                        decisions: state.decisions.mark(),
                        members: state.members_tail.mark(),
                        coercions: state.coercions.mark(),
                        diagnostics: state.diagnostics.len(),
                        warnings: state.warnings.len(),
                    },
                )
            })
            .collect();
        let trail = self.infer.mark(&self.fulfill);

        ProbeMark {
            trail,
            node_types: self.node_types.open_probe(),
            selections: self.selections.len(),
            functions: self.functions.len(),
            lambdas: self.lambdas.len(),
            walked_declarations: self.walked_declarations.len(),
            declaration_types: self.declaration_types.len(),
            binding_types: self.binding_types.len(),
            symbol_variables: self.infer.symbol_variables.len(),
            expected_types: self.expected_types.open_probe(),
            pending: self.fulfill.work.len(),
            events: self.trace_events().len(),
            failures: self.fulfill.failures.len(),
            modules,
        }
    }

    /// End one probe, rolling its inference state back.
    fn end_probe(&mut self, mark: ProbeMark) -> CompilerResult<()> {
        let ProbeMark {
            trail,
            node_types,
            selections,
            functions,
            lambdas,
            walked_declarations,
            declaration_types,
            binding_types,
            symbol_variables,
            expected_types,
            pending,
            events,
            failures,
            modules,
        } = mark;

        // roll inference back, poisoning undone allocations
        let poison = self.intern_type(dir::Type::Error)?;
        self.infer.rollback(trail, poison, &mut self.fulfill)?;

        // close the probe's type and body tables
        self.node_types.close_probe(node_types);
        self.selections.truncate(selections);
        self.functions.truncate(functions);
        self.lambdas.truncate(lambdas);
        self.walked_declarations.truncate(walked_declarations);
        while self.declaration_types.len() > declaration_types {
            self.declaration_types.pop();
        }
        while self.binding_types.len() > binding_types {
            self.binding_types.pop();
        }
        while self.infer.symbol_variables.len() > symbol_variables {
            self.infer.symbol_variables.pop();
        }
        self.expected_types.close_probe(expected_types);

        // cancel the probe's pending work, drop its trace events and failures
        self.fulfill.cancel_work_from(pending);
        if let Some(trace) = &mut self.trace {
            trace.events.truncate(events);
        }
        self.fulfill.failures.truncate(failures);

        // drop the probe's per module segments
        for (module, mark) in modules {
            if let Some(state) = self.module_maybe_mut(module) {
                state.decisions.truncate_to(mark.decisions);
                state.members_tail.truncate_to(mark.members);
                state.coercions.truncate_to(mark.coercions);
                state.diagnostics.truncate(mark.diagnostics);
                state.warnings.truncate(mark.warnings);
            }
        }

        Ok(())
    }

    /// Commit one successful probe.
    fn commit_probe(&mut self, mark: ProbeMark) -> CompilerResult<()> {
        self.infer.commit(mark.trail);

        Ok(())
    }
}
