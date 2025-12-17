use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a goto_implementation test.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    let Some(exp) = expectation else {
        return TestResult::Skipped {
            reason: "no goto_implementation expectation provided".to_string(),
        };
    };

    // get cursor position from the session
    let Some(cursor) = session.markers.cursor0() else {
        return TestResult::Failed {
            message: "no cursor ($0) marker in test file".to_string(),
        };
    };

    // run the query
    let result = query::goto_implementation(&session.session, session.file_id, cursor.offset);

    let Some(result) = result else {
        return TestResult::Failed {
            message: "goto_implementation returned None".to_string(),
        };
    };

    // parse expected count from expectation content
    let expected_content = exp.content.trim();
    let expected_count: usize = expected_content.parse().unwrap_or(0);

    if result.locations.len() != expected_count {
        return TestResult::Failed {
            message: format!(
                "expected {} implementations, found {}",
                expected_count,
                result.locations.len()
            ),
        };
    }

    TestResult::Passed
}
