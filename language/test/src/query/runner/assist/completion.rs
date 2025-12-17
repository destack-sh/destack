use destack_workspace::query::{self, CompletionTrigger};

use crate::harness::TestResult;
use crate::query::{ExpectedCompletion, QueryExpectation, QueryTestSession};

/// Run a completion test.
///
/// Supports both markdown format (query block) and inline format (@completion).
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    // if we have a markdown expectation, use it
    if let Some(exp) = expectation {
        return run_with_expectation(session, exp);
    }

    // fallback: use inline @completion expectations
    let expectations = &session.markers.expectations.completions;

    if expectations.is_empty() {
        return TestResult::Skipped {
            reason: "no completion expectations defined".to_string(),
        };
    }

    for (cursor_idx, expected) in expectations {
        let Some(cursor) = session.markers.cursor(*cursor_idx) else {
            return TestResult::Failed {
                message: format!("cursor ${cursor_idx} not found in source"),
            };
        };

        let completions = query::completions(
            &session.session,
            session.file_id,
            cursor.offset,
            CompletionTrigger::Invoked,
        );

        // check expected completions are present
        for exp in expected {
            let found = completions
                .iter()
                .any(|item| item.label == exp.label && kind_to_str(&item.kind) == exp.kind);

            if !found {
                let actual: Vec<_> = completions
                    .iter()
                    .map(|i| format!("{}({})", i.label, kind_to_str(&i.kind)))
                    .collect();
                return TestResult::Failed {
                    message: format!(
                        "completion '{}({})' not found at ${cursor_idx}\nactual: {actual:?}",
                        exp.label, exp.kind
                    ),
                };
            }
        }
    }

    TestResult::Passed
}

/// Run with markdown expectation: `query completion $0` with content listing expected items.
fn run_with_expectation(session: &QueryTestSession, exp: &QueryExpectation) -> TestResult {
    // parse cursor from target (e.g., "$0")
    let cursor_idx = parse_cursor_target(&exp.target);
    let Some(cursor) = session.markers.cursor(cursor_idx) else {
        return TestResult::Failed {
            message: format!("cursor ${cursor_idx} not found in source"),
        };
    };

    // parse expected completions from content (markdown list format)
    let ParsedExpectations { expected, excluded } = parse_completion_content(&exp.content);

    if expected.is_empty() && excluded.is_empty() {
        return TestResult::Skipped {
            reason: "no expected completions in query block".to_string(),
        };
    }

    let completions = query::completions(
        &session.session,
        session.file_id,
        cursor.offset,
        CompletionTrigger::Invoked,
    );

    // check expected completions are present
    for exp_item in &expected {
        let found = completions
            .iter()
            .any(|item| item.label == exp_item.label && kind_to_str(&item.kind) == exp_item.kind);

        if !found {
            let actual: Vec<_> = completions
                .iter()
                .map(|i| format!("{}: {}", i.label, kind_to_str(&i.kind)))
                .collect();
            return TestResult::Failed {
                message: format!(
                    "completion '{}: {}' not found\nactual: {:?}",
                    exp_item.label, exp_item.kind, actual
                ),
            };
        }
    }

    // check excluded completions are NOT present
    for exc_item in &excluded {
        let found = completions
            .iter()
            .any(|item| item.label == exc_item.label && kind_to_str(&item.kind) == exc_item.kind);

        if found {
            return TestResult::Failed {
                message: format!(
                    "completion '{}: {}' should NOT be present but was found",
                    exc_item.label, exc_item.kind
                ),
            };
        }
    }

    TestResult::Passed
}

/// Parse cursor target like "$0" or "$1" to cursor index.
fn parse_cursor_target(target: &str) -> usize {
    target
        .strip_prefix('$')
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// Parsed completion expectations.
struct ParsedExpectations {
    /// Completions that should be present.
    expected: Vec<ExpectedCompletion>,
    /// Completions that should NOT be present.
    excluded: Vec<ExpectedCompletion>,
}

/// Parse markdown list of completions: "- x: field\n- y: field".
/// Also supports exclusions with "! x: field" prefix.
fn parse_completion_content(content: &str) -> ParsedExpectations {
    let mut expected = Vec::new();
    let mut excluded = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // exclusion: "! x: field" or "- ! x: field"
        let is_exclusion = line.starts_with("! ") || line.starts_with("- ! ");
        let line = line
            .strip_prefix("- ! ")
            .or_else(|| line.strip_prefix("! "))
            .or_else(|| line.strip_prefix("- "));

        let Some(line) = line else { continue };
        let Some((label, kind)) = line.split_once(':') else {
            continue;
        };

        let completion = ExpectedCompletion {
            label: label.trim().to_string(),
            kind: kind.trim().to_string(),
            detail: None,
        };

        if is_exclusion {
            excluded.push(completion);
        } else {
            expected.push(completion);
        }
    }

    ParsedExpectations { expected, excluded }
}

fn kind_to_str(kind: &query::CompletionKind) -> &'static str {
    match kind {
        query::CompletionKind::Text => "text",
        query::CompletionKind::Method => "method",
        query::CompletionKind::Function => "function",
        query::CompletionKind::Constructor => "constructor",
        query::CompletionKind::Field => "field",
        query::CompletionKind::Variable => "variable",
        query::CompletionKind::Class => "class",
        query::CompletionKind::Interface => "interface",
        query::CompletionKind::Module => "module",
        query::CompletionKind::Property => "property",
        query::CompletionKind::Unit => "unit",
        query::CompletionKind::Value => "value",
        query::CompletionKind::Enum => "enum",
        query::CompletionKind::Keyword => "keyword",
        query::CompletionKind::Snippet => "snippet",
        query::CompletionKind::Color => "color",
        query::CompletionKind::File => "file",
        query::CompletionKind::Reference => "reference",
        query::CompletionKind::Folder => "folder",
        query::CompletionKind::EnumMember => "enum_member",
        query::CompletionKind::Constant => "constant",
        query::CompletionKind::Struct => "struct",
        query::CompletionKind::Event => "event",
        query::CompletionKind::Operator => "operator",
        query::CompletionKind::TypeParameter => "type_parameter",
    }
}
