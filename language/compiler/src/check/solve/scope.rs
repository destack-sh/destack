use crate::CompilerResult;
use crate::check::{CheckState, DeferredCheck, InferenceScope, Pass, PendingWork, WalkState};

/// How far one fulfillment settles the scope's owned variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::check) enum Settle {
    /// Resolve what the bounds determine, keep the remainder open.
    Bounded,
    /// Complete remaining roots from defaults and literal widening.
    Complete,
}

impl WalkState<'_, '_> {
    /// Run one closure as its own inference scope, closing it after.
    pub(in crate::check) fn with_scope<T>(
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
pub(in crate::check) struct ScopeMark {
    /// The variables and mutations the scope owns.
    scope: InferenceScope,
    /// The constraint count at the open.
    constraints: usize,
    /// The retained failure count at the open.
    failures: usize,
}

impl CheckState<'_> {
    /// Run one closure as its own inference scope, closing it after.
    pub(in crate::check) fn with_scope<T>(
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
            constraints: self.infer.constraint_count(),
            failures: self.infer.failures.len(),
        }
    }

    /// Drive one scope's fulfillment to a fixpoint.
    pub(in crate::check) fn fulfill_scope(
        &mut self,
        scope: InferenceScope,
        settle: Settle,
    ) -> CompilerResult<()> {
        loop {
            // propagate constraints and judge unblocked pending work
            if self.solve_where_possible()? {
                continue;
            }

            // resolve components the accumulated bounds already determine
            if self.resolve_scope(scope)? {
                continue;
            }

            // force the work this scope alone blocks
            if self.settle_owned_pending(scope)? {
                continue;
            }

            // complete remaining roots from declared defaults and widening
            if settle == Settle::Complete && self.default_scope(scope)? {
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

        self.fulfill_scope(mark.scope, Settle::Complete)?;

        // leave declaration holes open for elaborate
        if self.pass == Pass::Declare {
            self.infer.scope_depth -= 1;

            return Ok(());
        }

        // force every remainder to a verdict as it stands
        self.infer.forcing = true;
        let forced = self.fulfill_scope(mark.scope, Settle::Complete);
        self.infer.forcing = false;
        forced?;

        // report the pass's failures, then poison what stayed open
        let explained = self.report_failures(mark.constraints, mark.failures)?;
        self.report_unresolved(mark.scope, &explained)?;
        self.infer.scope_depth -= 1;

        Ok(())
    }

    /// Close the inference one statement opened.
    pub(in crate::check) fn close_statement(
        &mut self,
        scope: InferenceScope,
    ) -> CompilerResult<()> {
        // skip statements that opened no inference
        if self.infer.variable_count() == scope.first_variable() && self.infer.pending.is_empty() {
            return Ok(());
        }

        self.fulfill_scope(scope, Settle::Complete)
    }

    /// Settle the pending work blocked only on owned variables.
    fn settle_owned_pending(&mut self, scope: InferenceScope) -> CompilerResult<bool> {
        // split the pending work into the items this scope owns and the rest
        let pending = std::mem::take(&mut self.infer.pending);
        let mut owned = Vec::new();
        let mut foreign = Vec::new();
        for work in pending {
            // hold selections and expectations until their variables solve
            let is_forceable = !matches!(
                work,
                PendingWork::Check(DeferredCheck::Infer { .. } | DeferredCheck::Expect { .. })
            );
            let blockers = self.pending_blockers(&work)?;
            let is_owned = is_forceable
                && !blockers.is_empty()
                && blockers.iter().all(|variable| scope.owns(*variable));
            match is_owned {
                true => owned.push(work),
                false => foreign.push(work),
            }
        }
        if owned.is_empty() {
            self.infer.pending = foreign;

            return Ok(false);
        }

        // force the owned items alone, then restore the foreign ones
        self.infer.pending = owned;
        self.infer.forcing = true;
        let progressed = self.solve_where_possible()?;
        self.infer.forcing = false;
        self.infer.pending.extend(foreign);

        Ok(progressed)
    }
}
