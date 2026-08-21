use destack_repository::ArtifactAttemptRecorder;

use crate::CompilerResult;
use crate::sema::{Check, CheckState, FallbackStage, InferenceScope, Pass, Wake, WalkState};

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
    /// The check count at the open.
    checks: usize,
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
            scope: InferenceScope::open(self.infer.variable_count()),
            checks: self.fulfill.checks.count(),
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

        // give stage-waiting work its chance at this settle stage
        self.fulfill.wake(Wake::Stage);

        // give every waiting check its decisive chance at the final settle
        if settle == Settle::Final {
            self.fulfill.wake_all();

            // drive every relation check still open
            let incomplete = self
                .fulfill
                .checks
                .iter()
                .filter_map(|(id, check)| matches!(check, Check::Relation(_)).then_some(id))
                .filter(|id| !self.fulfill.checks.is_complete(*id))
                .collect::<Vec<_>>();
            for id in incomplete {
                self.solve_relation(id, Settle::Final)?;
            }
        }

        loop {
            // propagate constraints and step unblocked pending work
            if self.solve_where_possible(settle)? {
                self.fulfill.wake(Wake::Stage);
                continue;
            }

            // resolve components the accumulated bounds already determine
            if self.resolve_scope(scope, stage)? {
                self.fulfill.wake(Wake::Stage);
                continue;
            }

            // complete remaining roots from declared defaults and widening
            if settle != Settle::Bounded && self.apply_scope_default(scope)? {
                self.fulfill.wake(Wake::Stage);
                continue;
            }

            break;
        }

        Ok(())
    }

    /// Close one inference scope.
    fn close_scope(&mut self, mark: ScopeMark) -> CompilerResult<()> {
        // resolve only the variables a nested close allocated, leaving defaults to the outermost
        if self.infer.scope_depth > 1 {
            self.fulfill_scope(mark.scope, Settle::Bounded)?;

            // a declaration root closes what it opened, nothing later refines it
            if self.pass == Pass::Declare {
                while self.resolve_scope(mark.scope, FallbackStage::Final)? {
                    self.fulfill_scope(mark.scope, Settle::Bounded)?;
                }
            }
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
        let explained = self.report_failures(mark.checks, mark.failures)?;
        self.report_unresolved(mark.scope, &explained)?;

        // step the remainder over the poisoned holes
        self.fulfill_scope(mark.scope, Settle::Final)
    }

    /// Close the inference one statement opened.
    pub(in crate::sema) fn close_statement(&mut self, scope: InferenceScope) -> CompilerResult<()> {
        // skip statements that opened no inference
        if self.infer.variable_count() == scope.first_variable() && !self.fulfill.has_open_work() {
            return Ok(());
        }

        self.fulfill_scope(scope, Settle::Complete)
    }
}
