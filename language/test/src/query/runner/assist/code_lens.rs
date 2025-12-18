use destack_workspace::query;

use crate::harness::TestResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a code_lens test.
///
/// The expectation format is a list of lenses, one per line:
/// ```text
/// <lens_title>
/// ```
///
/// For example:
/// ```text
/// 2 references
/// 1 implementation
/// ▶ Run test_foo
/// ```
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    let Some(exp) = expectation else {
        return TestResult::Skipped {
            reason: "no code_lens expectation defined".to_string(),
        };
    };

    let lenses = query::code_lenses(&session.session, session.file_id);
    run_with_expectation(exp, &lenses)
}

/// Run with markdown expectation.
fn run_with_expectation(exp: &QueryExpectation, lenses: &[query::CodeLens]) -> TestResult {
    let content = exp.content.trim();

    // empty expectation with no lenses is a pass
    if content.is_empty() && lenses.is_empty() {
        return TestResult::Passed;
    }

    // "<none>" means we expect no lenses
    if content == "<none>" {
        return if lenses.is_empty() {
            TestResult::Passed
        } else {
            let formatted = format_lenses(lenses);
            TestResult::Failed {
                message: format!("code_lenses expected no lenses, got:\n{formatted}"),
            }
        };
    }

    // parse expected lens titles
    let expected: Vec<&str> = content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();

    // get actual lens titles
    let actual: Vec<String> = lenses.iter().map(|l| l.title()).collect();

    // compare counts
    if actual.len() != expected.len() {
        let formatted = format_lenses(lenses);
        return TestResult::Failed {
            message: format!(
                "code_lenses count mismatch: expected {}, got {}\nExpected:\n{}\nActual:\n{}",
                expected.len(),
                actual.len(),
                expected.join("\n"),
                formatted
            ),
        };
    }

    // compare each lens title
    for (i, (exp_title, act_title)) in expected.iter().zip(actual.iter()).enumerate() {
        if *exp_title != act_title {
            return TestResult::Failed {
                message: format!("lens {i} mismatch: expected '{exp_title}', got '{act_title}'"),
            };
        }
    }

    TestResult::Passed
}

/// Format lenses for error messages.
fn format_lenses(lenses: &[query::CodeLens]) -> String {
    lenses
        .iter()
        .map(|l| l.title())
        .collect::<Vec<_>>()
        .join("\n")
}
