use destack_core::StringPool;
use destack_source::{DiagnosticCollection, FileId, Span};

use crate::parse::{ParseOptions, Parser};
use crate::{CommentSpan, MirFormatOptions, Tree, format_mir};

/// Assert that one MIR node matches a pattern.
#[macro_export]
macro_rules! assert_node {
    ($tree:expr, $id:expr, $pat:pat_param) => {{
        #[allow(unreachable_patterns)]
        match $tree.get($id) {
            $pat => {}
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
    ($tree:expr, $id:expr, $pat:pat_param => $body:block) => {{
        #[allow(unreachable_patterns)]
        match $tree.get($id) {
            $pat => $body,
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
    ($node:expr, $pat:pat_param => $body:block) => {{
        #[allow(unreachable_patterns)]
        match $node {
            $pat => $body,
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
    ($node:expr, $pat:pat_param) => {{
        #[allow(unreachable_patterns)]
        match $node {
            $pat => {}
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
}

/// A test wrapper for MIR parsing.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TestParser<'a> {
    /// The raw MIR fixture source.
    pub source: &'a str,
}

impl<'a> TestParser<'a> {
    /// Create a new parser test fixture.
    pub(crate) fn new(source: &'a str) -> Self {
        Self { source }
    }

    /// Parse one fixture and require no parse errors.
    pub(crate) fn parse(self) -> (Tree, StringPool) {
        Parser::parse(FileId::new(0), self.source, ParseOptions::default())
            .finish()
            .expect("parse failed")
    }

    /// Parse one fixture and keep recovery diagnostics.
    pub(crate) fn parse_with_diagnostics(self) -> (Tree, DiagnosticCollection) {
        let parsed = Parser::parse(FileId::new(0), self.source, ParseOptions::default());
        let (tree, _, diagnostics) = parsed.into_parts();

        (tree, diagnostics)
    }

    /// Parse one fixture and return only the tree.
    pub(crate) fn tree(self) -> Tree {
        let (tree, _) = self.parse();
        tree
    }

    /// Assert one canonical format result.
    pub(crate) fn assert_format(self, expected: &str) {
        let (tree, strings) = self.parse();
        let output = format_mir(&tree, &strings, MirFormatOptions::default());

        assert_eq!(expected.trim(), output.trim(), "formatted output mismatch");
    }

    /// Build one span for the first matching text.
    pub(crate) fn span(self, text: &str) -> Span {
        let start = self.source.find(text).unwrap() as u32;
        Span::at(FileId::new(0), start, text.len() as u32)
    }

    /// Build one span for the first matching text inside one scope.
    pub(crate) fn span_in(self, scope: &str, text: &str) -> Span {
        let scope_start = self.source.find(scope).unwrap();
        let text_start = scope.find(text).unwrap();
        let start = (scope_start + text_start) as u32;

        Span::at(FileId::new(0), start, text.len() as u32)
    }

    /// Build one span for one matching text occurrence inside one scope.
    pub(crate) fn span_in_after(self, scope: &str, text: &str, occurrence: usize) -> Span {
        let scope_start = self.source.find(scope).unwrap();
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
}

/// Build one span for the first matching text.
pub(super) fn span_for_text(source: &str, text: &str) -> Span {
    TestParser::new(source).span(text)
}

/// Build one span for the first matching text inside one scope.
pub(super) fn span_for_text_in(source: &str, scope: &str, text: &str) -> Span {
    TestParser::new(source).span_in(scope, text)
}

/// Build one span for one matching text occurrence inside one scope.
pub(super) fn span_for_text_in_after(
    source: &str,
    scope: &str,
    text: &str,
    occurrence: usize,
) -> Span {
    TestParser::new(source).span_in_after(scope, text, occurrence)
}

/// Extract comment text for one parsed comment slice.
pub(super) fn comment_texts(comments: &[CommentSpan]) -> Vec<&str> {
    comments
        .iter()
        .map(|comment| comment.text.as_str())
        .collect()
}
