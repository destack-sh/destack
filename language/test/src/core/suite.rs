use std::collections::HashSet;
use std::time::Duration;

use super::{Case, CaseResult, RunContext, RunOptions};

/// One suite that can be discovered and run by the shared harness.
pub trait Suite: Send + Sync {
    /// Return the suite name used for display.
    fn name(&self) -> &'static str;

    /// Return the plural noun for discovered tests in this suite.
    fn case_noun(&self) -> &'static str {
        "tests"
    }

    /// Return whether this suite can execute cases in parallel.
    fn runs_in_parallel(&self) -> bool {
        true
    }

    /// Discover all cases in this suite.
    fn discover(&self, options: &RunOptions) -> Vec<Case>;

    /// Run one case.
    fn run(&self, case: &Case, context: &RunContext<'_>) -> CaseResult;

    /// Return an optional per-case timeout for this suite.
    fn timeout(&self) -> Option<Duration> {
        None
    }

    /// Report additional suite specific summary information after the run.
    fn report(&self, _results: &[(Case, CaseResult)], _context: &RunContext<'_>) {}

    /// Return expected failures for this suite when available.
    fn expected_failures(&self, _options: &RunOptions) -> Option<&HashSet<String>> {
        None
    }
}
