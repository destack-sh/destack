use std::time::Duration;

use super::{RunContext, TestCase, TestOptions, TestResult};

/// A test suite that can be discovered and executed by the shared harness.
pub trait Suite: Send + Sync {
    /// The suite name used for display and test naming.
    fn name(&self) -> &'static str;

    /// Discover all cases in this suite.
    fn discover(&self, options: &TestOptions) -> Vec<TestCase>;

    /// Run a single case.
    fn run(&self, case: &TestCase, context: &RunContext<'_>) -> TestResult;

    /// Return an optional per-test timeout for this suite.
    fn timeout(&self) -> Option<Duration> {
        None
    }

    /// Report additional suite-specific summary information after the run.
    fn report(&self, _results: &[(TestCase, TestResult)], _context: &RunContext<'_>) {}
}
