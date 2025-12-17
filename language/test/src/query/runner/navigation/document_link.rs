use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a document_link test.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    let Some(exp) = expectation else {
        return TestResult::Skipped {
            reason: "no document_links expectation provided".to_string(),
        };
    };

    // run the query
    let links = query::document_links(&session.session, session.file_id);

    // parse expected count from expectation content
    let expected_content = exp.content.trim();
    let expected_count: usize = expected_content.parse().unwrap_or(0);

    if links.len() != expected_count {
        return TestResult::Failed {
            message: format!(
                "expected {} document links, found {}",
                expected_count,
                links.len()
            ),
        };
    }

    TestResult::Passed
}
