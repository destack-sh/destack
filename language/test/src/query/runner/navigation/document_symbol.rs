use destack_workspace::query;
use destack_workspace::query::DocumentSymbol;

use crate::harness::TestResult;
use crate::query::{QueryExpectation, QueryTestSession};

/// Run a document_symbols test.
///
/// Tests that document symbols returns the expected symbols for a file.
pub fn run(session: &QueryTestSession, expectation: Option<&QueryExpectation>) -> TestResult {
    let Some(exp) = expectation else {
        return TestResult::Skipped {
            reason: "no document_symbols expectation".to_string(),
        };
    };

    let expected = exp.content.trim();

    // empty expectation is an error - must specify expected symbols
    if expected.is_empty() {
        let symbols = query::document_symbols(&session.session, session.file_id);
        let actual_names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        return TestResult::Failed {
            message: format!(
                "document_symbols expectation is empty, but query returned: {actual_names:?}"
            ),
        };
    }

    let symbols = query::document_symbols(&session.session, session.file_id);

    // check if we should verify count
    if let Some(count_str) = expected.strip_prefix("count:") {
        let expected_count: usize = count_str.trim().parse().unwrap_or(0);
        if symbols.len() != expected_count {
            return TestResult::Failed {
                message: format!(
                    "document_symbols returned {} symbols, expected {}",
                    symbols.len(),
                    expected_count
                ),
            };
        }
        return TestResult::Passed;
    }

    // collect all symbol names (including children with indentation)
    let actual_lines = format_symbols_hierarchical(&symbols, 0);

    // parse expected lines with indentation
    let expected_lines: Vec<(usize, &str)> = expected
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let indent = l.len() - l.trim_start().len();
            let name = l.trim();
            (indent, name)
        })
        .collect();

    // check each expected line against actual
    for (exp_indent, exp_name) in &expected_lines {
        let found = actual_lines.iter().any(|(indent, name)| {
            // match by name, allowing indent to be approximate (just needs to be child-level)
            name == exp_name && (*exp_indent == 0) == (*indent == 0)
        });

        if !found {
            let actual_formatted: Vec<String> = actual_lines
                .iter()
                .map(|(indent, name)| format!("{}{}", " ".repeat(*indent), name))
                .collect();
            return TestResult::Failed {
                message: format!(
                    "document_symbols missing expected symbol '{}', got:\n{}",
                    exp_name,
                    actual_formatted.join("\n")
                ),
            };
        }
    }

    TestResult::Passed
}

/// Format symbols hierarchically with indentation.
fn format_symbols_hierarchical(symbols: &[DocumentSymbol], indent: usize) -> Vec<(usize, String)> {
    let mut result = Vec::new();
    for symbol in symbols {
        result.push((indent, symbol.name.clone()));
        result.extend(format_symbols_hierarchical(&symbol.children, indent + 2));
    }
    result
}
