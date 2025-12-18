use destack_source::Span;
use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a code_actions test.
///
/// Verifies that code actions are available at the expected positions.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    let Some(exp) = expectation else {
        return TestResult::Skipped {
            reason: "no code_actions expectation defined".to_string(),
        };
    };

    run_with_expectation(session, exp)
}

/// Run with markdown expectation.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    let content = exp.content.trim();

    // get the full file range
    let range = Span::new(session.file_id, 0, session.source.len() as u32);
    let context = query::CodeActionContext::default();
    let actions = query::code_actions(&session.session, session.file_id, range, &context);

    // if content is a number, check count
    if let Ok(expected_count) = content.parse::<usize>() {
        if actions.len() != expected_count {
            return TestResult::Failed {
                message: format!(
                    "code_actions returned {} actions, expected {}",
                    actions.len(),
                    expected_count
                ),
            };
        }
        return TestResult::Passed;
    }

    // if content is "<none>", check for empty
    if content == "<none>" {
        if !actions.is_empty() {
            let action_titles: Vec<_> = actions.iter().map(|a| a.title.clone()).collect();
            return TestResult::Failed {
                message: format!(
                    "code_actions returned {} actions, expected none\nactions: {:?}",
                    actions.len(),
                    action_titles
                ),
            };
        }
        return TestResult::Passed;
    }

    // otherwise, check if any action title contains the expected text
    let found = actions.iter().any(|a| a.title.contains(content));
    if !found {
        let action_titles: Vec<_> = actions.iter().map(|a| a.title.clone()).collect();
        return TestResult::Failed {
            message: format!(
                "no code action with title containing '{}'\navailable: {:?}",
                content, action_titles
            ),
        };
    }

    TestResult::Passed
}
