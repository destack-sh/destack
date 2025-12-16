use destack_workspace::query::{self, CompletionTrigger};

use crate::harness::TestResult;
use crate::query::QueryTestSession;

/// Run a completion test.
pub fn run(session: &QueryTestSession) -> TestResult {
    let expectations = &session.markers.expectations.completions;

    if expectations.is_empty() {
        return TestResult::Skipped {
            reason: "no @completion expectations defined".to_string(),
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
