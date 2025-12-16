use crate::harness::TestResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a rename test.
///
/// Verifies that renaming a symbol updates all references correctly.
pub fn run(_session: &QueryTestSession, _expectation: Option<&QueryExpectation>) -> TestResult {
    todo!("#Incomplete: rename test runner")
}
