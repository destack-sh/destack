use destack_dir as dir;

use crate::CompilerResult;
use crate::sema::{BoundSide, CheckState, Settle, VariableKind, VariableState, Verdict};

/// Result of one candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::sema) enum CandidateOutcome<T, R> {
    /// The candidate applies.
    Accepted(T),
    /// The candidate is refused.
    Rejected(R),
}

impl CheckState<'_> {
    /// Decide one attempt over scratch variables, rolling back everything it writes.
    pub(in crate::sema) fn decide<T>(
        &mut self,
        attempt: impl FnOnce(&mut Self) -> CompilerResult<T>,
    ) -> CompilerResult<(T, Verdict)> {
        let scope = self.infer.variable_count();
        let checks = self.fulfill.checks.count();
        let failures = self.fulfill.failures.len();
        let symbol_variables = self.infer.symbol_variables.len();
        let filled_applications = self.infer.filled_applications.len();
        let declaration_types = self.declaration_types.len();
        let binding_types = self.binding_types.len();
        let functions = self.functions.len();
        let lambdas = self.lambdas.len();
        let owned_constructions = self.owned_constructions.len();
        let fresh_consts = self.fresh_consts.len();
        let scheduling = self.fulfill.scheduling();

        // mark the state the attempt rolls back to
        self.infer.snapshot(checks);

        // run the attempt and settle what its bounds decide
        let attempted = attempt(self).and_then(|value| {
            self.fulfill_scope(scope, Settle::Possible)?;

            Ok(value)
        });
        let verdict = attempted
            .as_ref()
            .ok()
            .map(|_| self.decision_verdict(scope, checks, failures))
            .transpose()?;

        // roll the decision back over everything it wrote, queued, woke, or failed
        for node in self.infer.rollback()? {
            self.node_types.remove(node);
        }
        self.fulfill.restore_scheduling(scheduling);
        self.fulfill.truncate(checks);
        self.fulfill.failures.truncate(failures);
        self.infer.symbol_variables.truncate(symbol_variables);
        self.infer.filled_applications.truncate(filled_applications);
        self.declaration_types.truncate(declaration_types);
        self.binding_types.truncate(binding_types);
        self.functions.truncate(functions);
        self.lambdas.truncate(lambdas);
        self.owned_constructions.truncate(owned_constructions);
        self.fresh_consts.truncate(fresh_consts);

        // leave a nested decision's variables dead, poisoning them from the outermost one
        let is_nested = self.infer.is_deciding();
        let error = self.intern_type(dir::Type::Error)?;
        for index in scope..self.infer.variable_count() {
            let variable = self.infer.variable_mut(dir::TypeVariableId(index as u32))?;
            if !variable.state.is_open() {
                continue;
            }
            match is_nested {
                true => variable.is_dead = true,
                false => variable.state = VariableState::Error(error),
            }
        }

        Ok((attempted?, verdict.unwrap_or(Verdict::Fails)))
    }

    /// Judge what one decision left: failures refuse it, open variables leave it undecided.
    fn decision_verdict(
        &mut self,
        scope: usize,
        checks: usize,
        failures: usize,
    ) -> CompilerResult<Verdict> {
        let has_failure = self
            .fulfill
            .checks
            .relation_failures_from(checks)
            .next()
            .is_some()
            || self.fulfill.failures.len() > failures;
        if has_failure {
            return Ok(Verdict::Fails);
        }

        // open memory slots elect later without undeciding the decision
        let mut is_open = false;
        for variable in self.open_scope_variables(scope)? {
            let opened = *self.infer.variable(variable)?;
            if matches!(opened.kind, VariableKind::Memory(_)) || opened.is_dead {
                continue;
            }
            // count a slot as decided by its default, its closed value, or its closed bounds
            let mut is_decided = self.infer.variable(variable)?.default.is_some();
            let sides: &[BoundSide] = match self.infer.variable(variable)?.parameter.is_some()
                || opened.kind.is_numeric()
            {
                true => &[BoundSide::Lower, BoundSide::Upper],
                false => &[BoundSide::Lower],
            };
            for side in sides {
                let bounds: Vec<_> = self
                    .infer
                    .variables
                    .side_bounds(variable, *side)?
                    .map(|bound| bound.ty)
                    .collect();
                for ty in bounds {
                    is_decided |= !self.type_flags(ty)?.has_variable();
                }
            }
            is_open |= !is_decided;
        }

        // report an open decision as ambiguous
        Ok(match is_open {
            true => Verdict::Ambiguous,
            false => Verdict::Holds,
        })
    }

    /// Decide one candidate.
    pub(in crate::sema) fn decide_candidate<T, R>(
        &mut self,
        attempt: impl FnOnce(&mut Self) -> CompilerResult<CandidateOutcome<T, R>>,
    ) -> CompilerResult<Verdict> {
        let (outcome, verdict) = self.decide(attempt)?;

        // report a rejected candidate as a failure
        Ok(match outcome {
            CandidateOutcome::Accepted(_) => verdict,
            CandidateOutcome::Rejected(_) => Verdict::Fails,
        })
    }

    /// Decide one deduction, keeping only a value deduced without failures.
    pub(in crate::sema) fn decide_deduction<R>(
        &mut self,
        attempt: impl FnOnce(&mut Self) -> CompilerResult<Option<R>>,
    ) -> CompilerResult<Option<R>> {
        let (deduced, verdict) = self.decide(attempt)?;

        Ok(deduced.filter(|_| verdict != Verdict::Fails))
    }
}
