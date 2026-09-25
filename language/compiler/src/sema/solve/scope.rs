use tspp_repository::ArtifactAttemptRecorder;

use crate::CompilerResult;
use crate::sema::{Check, CheckState, Pass, WalkState};

/// How far one fulfillment resolves the scope's owned variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::sema) enum Settle {
    /// Settle the variables their bounds decide, leaving the rest open.
    Possible,
    /// Settle every variable, the undecided ones through their fallbacks.
    All,
    /// Settle every variable at the end of a body, the undecided ones through their defaults.
    Final,
}

impl Settle {
    /// Return whether this stage settles every variable.
    pub(in crate::sema) fn settles(self) -> bool {
        !matches!(self, Self::Possible)
    }
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

    /// Run one closure as one body's inference scope, resolving its variables fully after.
    pub(in crate::sema) fn with_body_scope<T>(
        &mut self,
        scoped: impl FnOnce(&mut Self) -> CompilerResult<T>,
    ) -> CompilerResult<T> {
        let mark = self.check.open_scope();
        let value = scoped(self)?;
        self.check.close_body_scope(mark)?;

        Ok(value)
    }
}

/// The state one inference scope opened with.
#[derive(Debug, Clone, Copy)]
pub(in crate::sema) struct ScopeMark {
    /// The variables and mutations the scope owns.
    scope: usize,
    /// The check count at the open.
    checks: usize,
    /// The retained failure count at the open.
    failures: usize,
    /// The reported diagnostic count at the open.
    diagnostics: usize,
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

        // mark the state this scope rolls back to
        ScopeMark {
            scope: self.infer.variable_count(),
            checks: self.fulfill.checks.count(),
            failures: self.fulfill.failures.len(),
            diagnostics: self.module.diagnostics.len(),
        }
    }

    /// Drive one scope's fulfillment to a fixpoint.
    pub(in crate::sema) fn fulfill_scope(
        &mut self,
        scope: usize,
        stage: Settle,
    ) -> CompilerResult<()> {
        // propagate the bounds and resolve the variables the stage allows, to a fixpoint
        loop {
            if self.solve_where_possible(Settle::Possible)? {
                continue;
            }
            if self.resolve_scope(scope, stage)? {
                continue;
            }

            break;
        }
        if stage == Settle::Possible {
            return Ok(());
        }

        // settle every relation of this scope still open, then finish the fixpoint
        let fence = self.infer.decided_checks();
        self.fulfill.wake_all(fence);
        let incomplete = self
            .fulfill
            .checks
            .iter()
            .filter_map(|(id, check)| matches!(check, Check::Relation(_)).then_some(id))
            .filter(|id| id.index() >= fence && !self.fulfill.checks.is_complete(*id))
            .collect::<Vec<_>>();
        for id in incomplete {
            self.solve_relation(id, Settle::All)?;
        }
        while self.solve_where_possible(Settle::All)? || self.resolve_scope(scope, stage)? {}

        Ok(())
    }

    /// Run one closure as one body's inference scope, resolving its variables fully after.
    pub(in crate::sema) fn with_body_scope<T>(
        &mut self,
        scoped: impl FnOnce(&mut Self) -> CompilerResult<T>,
    ) -> CompilerResult<T> {
        let mark = self.open_scope();
        let value = scoped(self)?;
        self.close_body_scope(mark)?;

        Ok(value)
    }

    /// Close one body scope, resolving every variable it opened.
    fn close_body_scope(&mut self, mark: ScopeMark) -> CompilerResult<()> {
        self.settle_scope(mark.scope, Settle::Final)?;
        self.infer.scope_depth -= 1;

        Ok(())
    }

    /// Close one inference scope.
    fn close_scope(&mut self, mark: ScopeMark) -> CompilerResult<()> {
        // resolve the variables a nested close allocated, leaving defaults to the outermost
        if self.infer.scope_depth > 1 {
            let stage = match self.pass {
                Pass::Declare => Settle::All,
                _ => Settle::Possible,
            };
            self.settle_scope(mark.scope, stage)?;
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
        // complete the final components from their bounds as they stand
        self.fulfill_scope(mark.scope, Settle::Final)?;
        if self.pass == Pass::Declare {
            return Ok(());
        }

        // report the pass's failures, then poison what stayed open
        let explained = self.report_failures(mark.checks, mark.failures)?;
        self.report_unresolved(mark.scope, mark.diagnostics, &explained)
    }

    /// Settle one scope at one resolve stage.
    pub(in crate::sema) fn settle_scope(
        &mut self,
        scope: usize,
        stage: Settle,
    ) -> CompilerResult<()> {
        // fulfill the bounds, then complete the scope's own roots at the final stage
        self.fulfill_scope(scope, Settle::Possible)?;
        if stage.settles() {
            while self.resolve_scope(scope, Settle::All)? {
                self.fulfill_scope(scope, Settle::Possible)?;
            }
        }

        Ok(())
    }
}
