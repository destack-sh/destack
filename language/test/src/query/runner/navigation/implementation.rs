use crate::harness::TestResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a goto_implementation test.
///
/// NOTE #Incomplete: goto_implementation requires type system integration to find
/// implementations of interfaces or subclasses. Currently incomplete.
pub fn run(_session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    if expectation.is_some() {
        return TestResult::Skipped {
            reason: "goto_implementation not yet implemented".to_string(),
        };
    }

    TestResult::Skipped {
        reason: "no goto_implementation expectation provided".to_string(),
    }
}
