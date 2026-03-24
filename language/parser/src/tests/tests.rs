use std::sync::Arc;

use destack_ast::{NodeType, TokenType};
use destack_source::{File, FileId, FileType, LanguageType, Uri};

use crate::Parser;

/// A test wrapper for Parser.
#[derive(Debug)]
pub(crate) struct TestParser {
    pub file: Arc<File>,
    pub language: LanguageType,
}

impl TestParser {
    /// Create a new TestParser with default options.
    pub(crate) fn new(input: &str) -> Self {
        Self::new_with_options(input, LanguageType::default())
    }

    /// Create a new TestParser with custom options.
    pub(crate) fn new_with_options(input: &str, language: LanguageType) -> Self {
        let file_id = FileId::new(0);
        let file = File::from_text(
            file_id,
            "<string>".to_string(),
            Uri::from_string("<string>"),
            None,
            FileType::Destack,
            input.to_string(),
        );
        Self {
            file: Arc::new(file),
            language,
        }
    }

    /// Get a Parser for this test.
    pub(crate) fn prepare(&mut self) -> Parser {
        Parser::lex_file(self.file.clone(), self.language)
    }

    /// Assert the leaf parser errors by node type, expected token, and source text.
    pub(crate) fn assert_error_leaves(
        &self,
        parser: &Parser,
        expected_errors: &[(Option<NodeType>, Option<TokenType>, &str)],
    ) {
        let actual_errors: Vec<_> = parser
            .errors
            .iter()
            .map(|error| {
                let (span, node_type, token_type) = error.leaf_content();
                (node_type, token_type, parser.get_span_str(span).to_owned())
            })
            .collect();

        assert_eq!(
            actual_errors.len(),
            expected_errors.len(),
            "actual parser errors: {actual_errors:#?}",
        );

        for (
            (actual_node_type, actual_token, actual_text),
            (expected_node_type, expected_token, expected_text),
        ) in actual_errors.iter().zip(expected_errors.iter())
        {
            assert_eq!(
                *actual_node_type, *expected_node_type,
                "actual parser errors: {actual_errors:#?}"
            );
            assert_eq!(
                *actual_token, *expected_token,
                "actual parser errors: {actual_errors:#?}"
            );
            assert_eq!(
                actual_text, expected_text,
                "actual parser errors: {actual_errors:#?}"
            );
        }
    }

    /// Assert that the parser produced no diagnostics.
    pub(crate) fn assert_no_errors(&self, parser: &Parser) {
        assert!(parser.errors.is_empty(), "{:#?}", parser.errors);
    }
}

/// Assert that `tree.get(id)` matches `$pat`.
/// If a body is provided (`=> { ... }`), it runs with the pattern bindings.
///
/// Examples:
/// ```
/// assert_node!(tree, id, Pattern::Wildcard);
/// assert_node!(tree, id, Pattern::Pointer { mutability, target } => {
///     assert_eq!(*mutability, Mutability::Mutable);
///     assert_node!(tree, *target, Pattern::Wildcard);
/// });
/// ```
#[macro_export]
macro_rules! assert_node {
    // `tree.get(id)` matches a pattern, no body.
    // e.g., `assert_node!(tree, id, Pattern::Wildcard);`
    ($tree:expr, $id:expr, $pat:pat_param) => {{
        #[allow(unreachable_patterns)]
        match $tree.get($id) {
            $pat => {}
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
    // `tree.get(id)` matches a pattern, then run a block with the bindings.
    // e.g., `assert_node!(tree, id, Pattern::Pointer { mutability, target } => { /* ... */ });`
    ($tree:expr, $id:expr, $pat:pat_param => $body:block) => {{
        #[allow(unreachable_patterns)]
        match $tree.get($id) {
            $pat => $body,
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
    // Already-resolved node matches a pattern, run a block.
    // e.g., `assert_node!(node_ref, Pattern::Tuple { fields } => { /* ... */ });`
    ($node:expr, $pat:pat_param => $body:block) => {{
        #[allow(unreachable_patterns)]
        match $node {
            $pat => $body,
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
    // Already-resolved node matches a pattern, no body.
    // e.g., `assert_node!(node_ref, Pattern::Rest);`
    ($node:expr, $pat:pat_param) => {{
        #[allow(unreachable_patterns)]
        match $node {
            $pat => {}
            other => panic!("expected `{}`, got {other:?}", stringify!($pat)),
        }
    }};
}

/// Assert a `StringId` directly against an expected string.
#[macro_export]
macro_rules! assert_string {
    ($parser:expr, $id:expr, $expected:expr) => {{
        let maybe_id: Option<destack_core::StringId> = ::core::convert::Into::into($id);
        let got = maybe_id
            .map(|string_id| $parser.strings.get(string_id).to_string())
            .unwrap_or_default();
        assert_eq!(got, $expected, "expected string");
    }};
}

/// Assert a `Name` directly against an expected string.
#[macro_export]
macro_rules! assert_name {
    ($parser:expr, $name:expr, $expected:expr) => {{
        let got = $parser.strings.get($name.string()).to_string();
        assert_eq!(got, $expected, "expected name");
    }};
}

/// Assert a `Path` directly against an expected string.
#[macro_export]
macro_rules! assert_path {
    ($parser:expr, $path:expr, $expected:expr) => {{
        let path_str = $path
            .segments
            .iter()
            .map(|s| $parser.strings.get(*s).to_string())
            .collect::<Vec<_>>()
            .join(".");
        assert_eq!(path_str, $expected, "expected path");
    }};
}

/// Assert an "Expression::Path(path)" directly against an expected string.
#[macro_export]
macro_rules! assert_expression_path {
    ($parser:expr, $expr:expr, $expected:expr) => {{
        match $expr {
            ::destack_ast::Expression::Path {
                path,
                static_arguments: _,
            } => {
                assert_path!($parser, *path, $expected);
            }
            other => panic!("expected Expression::Path, got {other:?}"),
        }
    }};
}

/// Assert one comment trivia entry by style and normalized source payload.
#[macro_export]
macro_rules! assert_comment_trivia {
    ($parser:expr, $index:expr, $expected_style:expr, $expected_text:expr) => {{
        let trivia = &$parser.tree.comment_trivia()[$index];
        let comment = $parser.tree.get::<destack_ast::Comment>(trivia.comment);
        assert_eq!(comment.style, $expected_style, "expected comment style");

        let source = $parser.file.span_str(trivia.span);
        let got = destack_ast::normalize_comment_payload(source);
        assert_eq!(got.as_ref(), $expected_text, "expected comment text");
    }};
}
