use destack_dir as dir;

use crate::sema::{
    BodyState, Check, CheckEvent, CheckId, CheckOutcome, CheckState, Settle, TrailMark, Verdict,
    WorkState,
};
use crate::{CompilerError, CompilerResult};

/// Result of one speculative candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::sema) enum CandidateOutcome<T, R> {
    /// The candidate applies.
    Accepted(T),
    /// The candidate is refused.
    Rejected(R),
}

/// Result of running one candidate attempt.
enum CandidateAttempt<T, R> {
    /// The attempt produced an outcome.
    Outcome(CandidateOutcome<T, R>),
    /// One constraint solved during the attempt failed, rejecting the candidate.
    Failed,
}

/// Check state mark before one probe.
#[derive(Debug)]
pub(in crate::sema) struct ProbeMark {
    /// The inference trail mark before the probe.
    trail: TrailMark,
    /// The node type count before the probe.
    node_types: usize,
    /// The substitution memo count before the probe.
    substitutions: usize,
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
    /// The event count before the probe.
    events: usize,
    /// The failed check count before the probe.
    failures: usize,
    /// The own module's segment marks before the probe.
    module: ModuleProbeMark,
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
    /// Flow segment mark before the probe.
    flows: dir::FlowMark,
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
    ) -> CompilerResult<Verdict> {
        let mark = self.check.open_probe();
        let outcome = self.attempt_candidate(&mark, &mut attempt);

        self.check.finish_probe(mark, outcome)
    }

    /// Probe one candidate under a rollback, fulfilling its constraints first.
    ///
    /// The fulfillment also steps pending work queued before the probe, and the rollback keeps
    /// that work completed, so only a caller that confirms the winner afterwards may probe
    /// this way.
    pub(in crate::sema) fn probe_candidate_fulfilling<T, R>(
        &mut self,
        mut attempt: impl FnMut(&mut Self) -> CompilerResult<CandidateOutcome<T, R>>,
    ) -> CompilerResult<Verdict> {
        let mark = self.check.open_probe();
        let outcome = self.evaluate_candidate(&mark, &mut attempt);

        self.check.finish_probe(mark, outcome)
    }

    /// Probe one deduction under a rollback, keeping only its refused value.
    pub(in crate::sema) fn probe_deduction<R>(
        &mut self,
        mut attempt: impl FnMut(&mut Self) -> CompilerResult<CandidateOutcome<(), R>>,
    ) -> CompilerResult<Option<R>> {
        // pull the deduced value out before the rollback consumes the outcome
        let mark = self.check.open_probe();
        let outcome = self.attempt_candidate(&mark, &mut attempt);
        let (outcome, deduced): (CompilerResult<CandidateAttempt<(), ()>>, Option<R>) =
            match outcome {
                Ok(CandidateAttempt::Outcome(CandidateOutcome::Rejected(deduced))) => (
                    Ok(CandidateAttempt::Outcome(CandidateOutcome::Rejected(()))),
                    Some(deduced),
                ),
                Ok(CandidateAttempt::Outcome(CandidateOutcome::Accepted(()))) => (
                    Ok(CandidateAttempt::Outcome(CandidateOutcome::Rejected(()))),
                    None,
                ),
                Ok(CandidateAttempt::Failed) => (Ok(CandidateAttempt::Failed), None),
                Err(error) => (Err(error), None),
            };
        self.check.finish_probe(mark, outcome)?;

        Ok(deduced)
    }

    /// Run one candidate attempt, rejecting it on failed constraints.
    fn attempt_candidate<T, R>(
        &mut self,
        mark: &ProbeMark,
        attempt: &mut impl FnMut(&mut Self) -> CompilerResult<CandidateOutcome<T, R>>,
    ) -> CompilerResult<CandidateAttempt<T, R>> {
        let outcome = attempt(self)?;

        Ok(self.judge_attempt(mark, outcome, false))
    }

    /// Run one candidate attempt and fulfill the relations it queued to a fixpoint, rejecting
    /// it on failed constraints.
    fn evaluate_candidate<T, R>(
        &mut self,
        mark: &ProbeMark,
        attempt: &mut impl FnMut(&mut Self) -> CompilerResult<CandidateOutcome<T, R>>,
    ) -> CompilerResult<CandidateAttempt<T, R>> {
        let outcome = attempt(self)?;

        // fulfill the candidate's own scope to a fixpoint
        let scope = mark.trail.inference_scope();
        self.check.fulfill_scope(scope, Settle::Complete)?;

        // re-solve the stalled pending set alongside fresh allocations
        let mut relations: Vec<CheckId> = Vec::new();
        for (id, check) in self.check.fulfill.checks.iter() {
            let is_open = self.check.fulfill.checks.state(id) != WorkState::Done;
            if is_open && matches!(check, Check::Relation(_)) {
                relations.push(id);
            }
        }
        for (id, check) in self
            .check
            .fulfill
            .checks
            .iter()
            .skip(mark.trail.check_count())
        {
            if matches!(check, Check::Relation(_)) && !relations.contains(&id) {
                relations.push(id);
            }
        }
        let mut has_failed_relation = false;
        for id in relations {
            self.check.solve_relation(id, Settle::Complete)?;
            if let Some(outcome) = self.check.fulfill.checks.result(id)? {
                has_failed_relation |= matches!(outcome, CheckOutcome::Fails(_));
            }
        }

        Ok(self.judge_attempt(mark, outcome, has_failed_relation))
    }

    /// Reject one attempt whose solved relation checks failed.
    fn judge_attempt<T, R>(
        &self,
        mark: &ProbeMark,
        outcome: CandidateOutcome<T, R>,
        has_failed_relation: bool,
    ) -> CandidateAttempt<T, R> {
        let has_failure = self
            .check
            .fulfill
            .checks
            .relation_failures_from(mark.trail.check_count())
            .next()
            .is_some()
            || self.check.fulfill.failures.len() > mark.failures;

        if has_failure || has_failed_relation {
            CandidateAttempt::Failed
        } else {
            CandidateAttempt::Outcome(outcome)
        }
    }

    /// Probe one candidate, describing a rejection before the rollback.
    pub(in crate::sema) fn probe_candidate_describing<T, R>(
        &mut self,
        mut attempt: impl FnMut(&mut Self) -> CompilerResult<CandidateOutcome<T, R>>,
        describe: impl FnOnce(&mut Self, &R) -> CompilerResult<String>,
    ) -> CompilerResult<(Verdict, Option<String>)> {
        self.check.counters.selection_probes += 1;
        let mark = self.check.open_probe();
        let outcome = self.evaluate_candidate(&mark, &mut attempt);

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
        let outcome = match self.attempt_candidate(&mark, &mut attempt) {
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
    ) -> CompilerResult<Verdict> {
        let mark = self.open_probe();
        let outcome = attempt(self).map(CandidateAttempt::Outcome);

        self.finish_probe(mark, outcome)
    }

    /// Probe one relation judgment under an inference-only rollback.
    pub(in crate::sema) fn probe_relation(
        &mut self,
        attempt: impl FnOnce(&mut Self) -> CompilerResult<bool>,
    ) -> CompilerResult<Verdict> {
        self.counters.probes += 1;
        self.record_event(CheckEvent::ProbeStarted {
            variables: self.infer.variable_count(),
        });
        let trail = self.infer.mark(&self.fulfill);
        let failures = self.fulfill.failures.len();

        let accepted = match attempt(self) {
            Ok(accepted) => accepted,
            Err(error) => {
                let poison = self.intern_type(dir::Type::Error)?;
                self.infer.rollback(trail, poison, &mut self.fulfill)?;
                self.fulfill.failures.truncate(failures);

                return Err(error);
            }
        };

        // reject failed judgments and roll their inference back
        let has_failure = !accepted
            || self
                .fulfill
                .checks
                .relation_failures_from(trail.check_count())
                .next()
                .is_some()
            || self.fulfill.failures.len() > failures;
        if has_failure {
            let poison = self.intern_type(dir::Type::Error)?;
            self.infer.rollback(trail, poison, &mut self.fulfill)?;
            self.fulfill.failures.truncate(failures);
            self.record_event(CheckEvent::ProbeFinished {
                verdict: Some(Verdict::Fails),
            });

            return Ok(Verdict::Fails);
        }

        // keep the judgment indeterminate while its variables stay open
        let verdict = match self
            .open_scope_variables(trail.inference_scope())?
            .is_empty()
        {
            true => Verdict::Holds,
            false => Verdict::Ambiguous,
        };
        self.infer.commit(trail);
        self.record_event(CheckEvent::ProbeFinished {
            verdict: Some(verdict),
        });

        Ok(verdict)
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
            CandidateOutcome::Rejected(_) => Ok(Verdict::Fails),
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
            Verdict::Holds | Verdict::Ambiguous => {
                let CandidateOutcome::Accepted(selected) = outcome else {
                    self.end_probe(mark)?;

                    return Err(CompilerError::Internal {
                        message: "confirmed candidate has no accepted outcome".into(),
                    });
                };
                self.record_event(CheckEvent::ProbeFinished {
                    verdict: Some(verdict),
                });
                self.commit_probe(mark)?;

                Ok(Some(selected))
            }
            Verdict::Fails => {
                self.reject_probe(mark)?;

                Ok(None)
            }
        }
    }

    /// Close one probe whose solved constraints failed.
    fn reject_probe(&mut self, mark: ProbeMark) -> CompilerResult<Verdict> {
        self.record_event(CheckEvent::ProbeFinished {
            verdict: Some(Verdict::Fails),
        });
        self.end_probe(mark)?;

        Ok(Verdict::Fails)
    }

    /// Map one attempted outcome onto its verdict, rolling the probe back.
    fn finish_probe<T, R>(
        &mut self,
        mark: ProbeMark,
        outcome: CompilerResult<CandidateAttempt<T, R>>,
    ) -> CompilerResult<Verdict> {
        let verdict = match outcome {
            Ok(CandidateAttempt::Outcome(CandidateOutcome::Accepted(_))) => {
                self.accepted_verdict(&mark)
            }
            Ok(CandidateAttempt::Outcome(CandidateOutcome::Rejected(_)))
            | Ok(CandidateAttempt::Failed) => Ok(Verdict::Fails),
            Err(error) => Err(error),
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
    fn accepted_verdict(&mut self, mark: &ProbeMark) -> CompilerResult<Verdict> {
        let scope = mark.trail.inference_scope();
        let has_failure = self
            .fulfill
            .checks
            .relation_failures_from(mark.trail.check_count())
            .next()
            .is_some()
            || self.fulfill.failures.len() > mark.failures;
        if has_failure {
            return Ok(Verdict::Fails);
        }

        // keep the candidate indeterminate while its variables stay open
        if !self.open_scope_variables(scope)?.is_empty() {
            return Ok(Verdict::Ambiguous);
        }

        Ok(Verdict::Holds)
    }

    /// Begin one probe, recording its start event.
    fn open_probe(&mut self) -> ProbeMark {
        self.counters.probes += 1;
        self.record_event(CheckEvent::ProbeStarted {
            variables: self.infer.variable_count(),
        });

        let module = ModuleProbeMark {
            decisions: self.module.decisions.mark(),
            members: self.module.members_tail.mark(),
            coercions: self.module.coercions.mark(),
            flows: self.module.flows.mark(),
            diagnostics: self.module.diagnostics.len(),
            warnings: self.module.warnings.len(),
        };
        let trail = self.infer.mark(&self.fulfill);

        ProbeMark {
            trail,
            node_types: self.node_types.open_probe(),
            substitutions: self.substitutions.len(),
            functions: self.functions.len(),
            lambdas: self.lambdas.len(),
            walked_declarations: self.walked_declarations.len(),
            declaration_types: self.declaration_types.len(),
            binding_types: self.binding_types.len(),
            symbol_variables: self.infer.symbol_variables.len(),
            expected_types: self.expected_types.open_probe(),
            events: self.trace_events().len(),
            failures: self.fulfill.failures.len(),
            module,
        }
    }

    /// End one probe, rolling its inference state back.
    fn end_probe(&mut self, mark: ProbeMark) -> CompilerResult<()> {
        let ProbeMark {
            trail,
            node_types,
            substitutions,
            functions,
            lambdas,
            walked_declarations,
            declaration_types,
            binding_types,
            symbol_variables,
            expected_types,
            events,
            failures,
            module,
        } = mark;

        // roll inference back, poisoning undone allocations
        let poison = self.intern_type(dir::Type::Error)?;
        self.infer.rollback(trail, poison, &mut self.fulfill)?;

        // close the probe's type and body tables
        self.node_types.close_probe(node_types)?;
        self.substitutions.truncate(substitutions);
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
        self.expected_types.close_probe(expected_types)?;

        // drop the probe's trace events and failures
        if let Some(trace) = &mut self.trace {
            trace.events.truncate(events);
        }
        self.fulfill.failures.truncate(failures);

        // drop the probe's own module segments
        self.module.decisions.truncate_to(module.decisions);
        self.module.members_tail.truncate_to(module.members);
        self.module.coercions.truncate_to(module.coercions);
        self.module.flows.truncate_to(module.flows);
        self.module.diagnostics.truncate(module.diagnostics);
        self.module.warnings.truncate(module.warnings);

        Ok(())
    }

    /// Commit one successful probe.
    fn commit_probe(&mut self, mark: ProbeMark) -> CompilerResult<()> {
        self.infer.commit(mark.trail);

        Ok(())
    }
}
