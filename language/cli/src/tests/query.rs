use clap::Parser;
use std::path::PathBuf;

use tspp_source::{FileId, Span, Uri};
use tspp_workspace::{QueryCapture, QueryCaptureValue, QueryMatch, QueryPayload};

use crate::app::{Cli, Command};
use crate::command::query::QueryArgs;
use crate::common::{NodeTypeArg, ReportFormat};

/// Parse query patterns, files, predicates, and contextual kinds compositionally.
#[test]
fn test_parse_query_command() {
    let query = Cli::try_parse_from([
        "tspp",
        "query",
        "$CALLEE($URL, $$$ARGUMENTS)",
        "src/main.tspp",
        "--where",
        "$CALLEE == net.fetch",
        "--kind",
        "expression",
        "--output-format",
        "text",
        "--capture",
        "URL",
        "--with-filename",
        "--one-line",
    ])
    .expect("parse query command");
    let Command::Query(query) = query.command else {
        panic!("expected query command");
    };
    assert_eq!(query.pattern, "$CALLEE($URL, $$$ARGUMENTS)");
    assert_eq!(query.input.files, [PathBuf::from("src/main.tspp")]);
    assert_eq!(query.predicates, ["$CALLEE == net.fetch"]);
    assert_eq!(query.report.format(), ReportFormat::Text);
    assert_eq!(query.kind, Some(NodeTypeArg::Expression));
    assert_eq!(query.captures, ["URL"]);
    assert!(query.with_filename);
    assert!(query.one_line);

    let count = Cli::try_parse_from(["tspp", "query", "fetch($URL)", "src/main.tspp", "--count"])
        .expect("parse count query command");
    let Command::Query(count) = count.command else {
        panic!("expected query command");
    };
    assert!(count.count);
}

/// Render complete roots and projected captures in stable text forms.
#[test]
fn test_render_query_text() {
    let cwd = PathBuf::from("/workspace");
    let file = FileId::from_logical_str("src/main.tspp");
    let payload = QueryPayload {
        captures: vec!["MESSAGE".to_string()],
        matches: vec![
            QueryMatch {
                uri: Uri::from_string("file:///workspace/src/main.tspp"),
                path: Some(cwd.join("src/main.tspp")),
                span: Span::new(file, 0, 13),
                line: 1,
                column: 1,
                text: "todo(\"later\")".to_string(),
                captures: vec![QueryCapture {
                    name: "MESSAGE".to_string(),
                    values: vec![QueryCaptureValue {
                        span: Span::new(file, 5, 12),
                        text: "\"later\"".to_string(),
                    }],
                }],
            },
            QueryMatch {
                uri: Uri::from_string("file:///workspace/src/main.tspp"),
                path: Some(cwd.join("src/main.tspp")),
                span: Span::new(file, 15, 33),
                line: 2,
                column: 1,
                text: "todo(\n    message\n)".to_string(),
                captures: vec![QueryCapture {
                    name: "MESSAGE".to_string(),
                    values: vec![QueryCaptureValue {
                        span: Span::new(file, 25, 32),
                        text: "message".to_string(),
                    }],
                }],
            },
        ],
    };

    let roots = QueryArgs::render_text(payload.clone(), &cwd, &[], false, false)
        .expect("render selected roots");
    let one_line = QueryArgs::render_text(payload.clone(), &cwd, &[], false, true)
        .expect("render one line roots");
    let captures = QueryArgs::render_text(payload, &cwd, &["MESSAGE".to_string()], true, false)
        .expect("render capture projection");

    assert_eq!(
        roots,
        concat!(
            "src/main.tspp:1:1: todo(\"later\")\n",
            "src/main.tspp:2:1\n",
            "todo(\n",
            "    message\n",
            ")\n",
        )
    );
    assert_eq!(
        one_line,
        concat!(
            "src/main.tspp:1:1: todo(\"later\")\n",
            "src/main.tspp:2:1: todo(\\n    message\\n)\n",
        )
    );
    assert_eq!(captures, "src/main.tspp:\"later\"\nsrc/main.tspp:message\n");
}
