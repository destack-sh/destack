use crate::harness::TestResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a document_link test.
///
/// NOTE #Incomplete: document_links requires import resolution to create clickable links.
/// Currently incomplete.
pub fn run(_session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    if expectation.is_some() {
        return TestResult::Skipped {
            reason: "document_links not yet implemented".to_string(),
        };
    }

    TestResult::Skipped {
        reason: "no document_links expectation provided".to_string(),
    }
}
