use destack_repository::ArtifactAttemptRecorder;

use crate::CompilerResult;
use crate::sema::{CheckState, FallbackStage, InferenceScope, Pass, WalkState};

/// How far one fulfillment settles the scope's owned variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum Settle {
    /// Resolve what the bounds determine, keep the remainder open.
    Bounded,
    /// Complete remaining roots from defaults and literal widening.
    Complete,
    /// Complete every component from its bounds as they stand.
    Final,
}

impl WalkState<'_, '_> {
    /// Run one closure as its own inference scope, closing it after.
    pub(in crate::sema) fn with_scope<T>(
        &mut self,
        scoped: impl FnOnce(&mut Self) -> CompilerResult<T>,
    ) -> CompilerResult<T> {
        let mark = self.check.open_scope();
        let value = scoped(self)?;
        self.check.close_scope(mark)?;

        Ok(value)
    }
}

/// The state one inference scope opened with.
#[derive(Debug, Clone, Copy)]
pub(in crate::sema) struct ScopeMark {
    /// The variables and mutations the scope owns.
    scope: InferenceScope,
    /// The constraint count at the open.
    constraints: usize,
    /// The retained failure count at the open.
    failures: usize,
}

impl CheckState<'_> {
    /// Run one closure as its own inference scope, closing it after.
    pub(in crate::sema) fn with_scope<T>(
        &mut self,
        scoped: impl FnOnce(&mut Self) -> CompilerResult<T>,
    ) -> CompilerResult<T> {
        let mark = self.open_scope();
        let value = scoped(self)?;
        self.close_scope(mark)?;

        Ok(value)
    }

    /// Open one inference scope.
    fn open_scope(&mut self) -> ScopeMark {
        self.infer.scope_depth += 1;

        ScopeMark {
            scope: InferenceScope::open(self.infer.variable_count(), self.infer.trail.len()),
            constraints: self.fulfill.constraint_count(),
            failures: self.fulfill.failures.len(),
        }
    }

    /// Drive one scope's fulfillment to a fixpoint.
    pub(in crate::sema) fn fulfill_scope(
        &mut self,
        scope: InferenceScope,
        settle: Settle,
    ) -> CompilerResult<()> {
        // resolve components at the stage the settle point allows
        let stage = match settle {
            Settle::Bounded | Settle::Complete => FallbackStage::Bounded,
            Settle::Final => FallbackStage::Final,
        };

        // give parked and stalled work its decisive chance at the final settle
        if settle == Settle::Final {
            self.fulfill.requeue_parked();
            self.fulfill.requeue_stalled();

            // drive every constraint that never reached a verdict
            let incomplete = self
                .fulfill
                .constraints
                .iter()
                .map(|(id, _)| id)
                .filter(|id| !self.fulfill.constraints.is_complete(*id))
                .collect::<Vec<_>>();
            for id in incomplete {
                self.solve_constraint(id, Settle::Final)?;
            }
        }

        loop {
            // propagate constraints and step unblocked pending work
            if self.solve_where_possible(settle)? {
                self.fulfill.requeue_parked();
                continue;
            }

            // resolve components the accumulated bounds already determine
            if self.resolve_scope(scope, stage)? {
                self.fulfill.requeue_parked();
                continue;
            }

            // complete remaining roots from declared defaults and widening
            if settle != Settle::Bounded && self.default_scope(scope)? {
                self.fulfill.requeue_parked();
                continue;
            }

            break;
        }

        Ok(())
    }

    /// Close one inference scope.
    fn close_scope(&mut self, mark: ScopeMark) -> CompilerResult<()> {
        // an inner close leaves defaults and reporting to the outermost
        if self.infer.scope_depth > 1 {
            self.fulfill_scope(mark.scope, Settle::Bounded)?;
            self.infer.scope_depth -= 1;

            return Ok(());
        }

        // drain the pass's own scope, timed as one phase
        let recorder = self.recorder;
        ArtifactAttemptRecorder::breakdown_maybe(recorder, "drain", || self.drain_scope(mark))?;
        self.infer.scope_depth -= 1;

        Ok(())
    }

    /// Drain the outermost scope, reporting whatever stays open.
    fn drain_scope(&mut self, mark: ScopeMark) -> CompilerResult<()> {
        self.fulfill_scope(mark.scope, Settle::Complete)?;

        // leave declaration holes open for elaborate
        if self.pass == Pass::Declare {
            return Ok(());
        }

        // complete the final components from their bounds as they stand
        self.fulfill_scope(mark.scope, Settle::Final)?;

        // report the pass's failures, then poison what stayed open
        let explained = self.report_failures(mark.constraints, mark.failures)?;
        self.report_unresolved(mark.scope, &explained)?;

        // step the remainder over the poisoned holes
        self.fulfill_scope(mark.scope, Settle::Final)
    }

    /// Close the inference one statement opened.
    pub(in crate::sema) fn close_statement(
        &mut self,
        scope: InferenceScope,
    ) -> CompilerResult<()> {
        // skip statements that opened no inference
        if self.infer.variable_count() == scope.first_variable() && self.fulfill.open_work == 0 {
            return Ok(());
        }

        self.fulfill_scope(scope, Settle::Complete)
    }
}
