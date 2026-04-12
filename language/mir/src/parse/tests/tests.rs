use crate::parse::{ParseOptions, Parser};
use crate::{CommentSpan, MirFormatOptions, NodeTree, format_mir};
use destack_source::{FileId, Span};

/// Test parsing and re-formatting produces the same output.
pub(super) fn roundtrip(source: &str) {
    let source = source.trim();
    let (tree, strings) = Parser::parse(FileId::new(0), source, ParseOptions::default())
        .validate()
        .expect("parse failed");
    let output = format_mir(&tree, &strings, MirFormatOptions::default());
    let output = output.trim().to_string();

    let (roundtrip_tree, roundtrip_strings) =
        Parser::parse(FileId::new(0), &output, ParseOptions::default())
            .validate()
            .expect("reparse failed");
    let roundtrip_output = format_mir(
        &roundtrip_tree,
        &roundtrip_strings,
        MirFormatOptions::default(),
    );

    assert_eq!(output, roundtrip_output.trim(), "roundtrip mismatch");
}

/// Parse one source snippet and assert its canonical formatted output.
pub(super) fn parse_and_format(source: &str, expected: &str) {
    let source = source.trim();
    let expected = expected.trim();
    let (tree, strings) = Parser::parse(FileId::new(0), source, ParseOptions::default())
        .validate()
        .expect("parse failed");
    let output = format_mir(&tree, &strings, MirFormatOptions::default());

    assert_eq!(expected, output.trim(), "formatted output mismatch");
}

/// Parse one MIR fixture.
pub(super) fn parse_fixture(source: &str) -> NodeTree {
    let (tree, _) = Parser::parse(FileId::new(0), source, ParseOptions::default())
        .validate()
        .expect("parse failed");

    tree
}

/// Parse one MIR fixture into parsed source and diagnostics.
pub(super) fn parse_fixture_source(
    source: &str,
) -> (NodeTree, destack_source::DiagnosticCollection) {
    let parsed = Parser::parse(FileId::new(0), source, ParseOptions::default());
    let (tree, _, diagnostics) = parsed.into_parts();

    (tree, diagnostics)
}

/// Extract comment text for one parsed comment slice.
pub(super) fn comment_texts(comments: &[CommentSpan]) -> Vec<&str> {
    comments
        .iter()
        .map(|comment| comment.text.as_str())
        .collect()
}

/// Build one span for the first matching text.
pub(super) fn span_for_text(source: &str, text: &str) -> Span {
    let start = source.find(text).unwrap() as u32;
    Span::at(FileId::new(0), start, text.len() as u32)
}

/// Build one span for the first matching text inside one scope.
pub(super) fn span_for_text_in(source: &str, scope: &str, text: &str) -> Span {
    let scope_start = source.find(scope).unwrap();
    let text_start = scope.find(text).unwrap();
    let start = (scope_start + text_start) as u32;

    Span::at(FileId::new(0), start, text.len() as u32)
}

/// Build one span for one matching text occurrence inside one scope.
pub(super) fn span_for_text_in_after(
    source: &str,
    scope: &str,
    text: &str,
    occurrence: usize,
) -> Span {
    let scope_start = source.find(scope).unwrap();
    let mut from = 0usize;
    let mut local_start = None;

    for _ in 0..=occurrence {
        let next = scope[from..].find(text).unwrap();
        local_start = Some(from + next);
        from = from + next + text.len();
    }

    let start = (scope_start + local_start.unwrap()) as u32;
    Span::at(FileId::new(0), start, text.len() as u32)
}
