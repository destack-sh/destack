use crate::Compiler;
use crate::check::solve::state::Solver;
use crate::check::{CheckResult, CheckState};

impl Compiler {
    /// Solve check constraints into semantic DIR table entries.
    pub(in crate::check) fn solve_check(&self, state: &mut CheckState<'_>) -> CheckResult<()> {
        let mut solver = Solver::new(state.constraints(), state.conditions(), state.obligations());
        solver.seed();
        solver.solve();

        Ok(())
    }
}
