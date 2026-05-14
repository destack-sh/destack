use std::sync::Arc;

use destack_core::StringPool;
use destack_dir::{
    AssignPattern, Block, Expression, LocalNodeId, NodeType, StringId, TokenType, TypeExpression,
    normalize_comment_payload,
};
use destack_source::{File, FileId, FileType, LanguageType, Uri};

use crate::Parser;

/// A test wrapper for Parser.
#[derive(Debug)]
pub(crate) struct TestParser {
    pub file: Arc<File>,
    pub language: LanguageType,
}

impl TestParser {
    /// Create a new TestParser with the default language.
    pub(crate) fn new(input: &str) -> Self {
        Self::new_with_language(input, LanguageType::default())
    }

    /// Create a new TestParser with a custom language.
    pub(crate) fn new_with_language(input: &str, language: LanguageType) -> Self {
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
        Parser::lex_file(
            self.file.clone(),
            self.language,
            Arc::new(StringPool::new()),
        )
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

/// Collect block expressions in source order for test assertions.
pub(crate) fn block_expression_ids(block: &Block) -> Vec<LocalNodeId<Expression>> {
    block.iter_expressions().collect()
}

/// Assert that `tree.get(id)` matches one node pattern, including type-space nodes.
///
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
    // tree nodes
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

    // resolved nodes
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

/// Collect one path-like string from an expression.
pub(crate) fn expression_path_string(
    parser: &Parser,
    expression: &impl ExpressionPathLike,
) -> Option<String> {
    let mut segments = Vec::new();
    expression.collect_path_segments(parser, &mut segments)?;

    Some(
        segments
            .into_iter()
            .map(|segment| parser.strings.get(segment).to_string())
            .collect::<Vec<_>>()
            .join("."),
    )
}

/// Collect path-like segments from one node.
pub(crate) trait ExpressionPathLike {
    /// Collect path-like segments from one node.
    fn collect_path_segments(&self, parser: &Parser, segments: &mut Vec<StringId>) -> Option<()>;
}

impl ExpressionPathLike for Expression {
    fn collect_path_segments(&self, parser: &Parser, segments: &mut Vec<StringId>) -> Option<()> {
        match self {
            Expression::Parenthesized { expression } => {
                let expression = parser.tree.get(*expression);
                expression.collect_path_segments(parser, segments)
            }
            Expression::Type { value } => {
                let expression = parser.tree.get(*value);
                expression.collect_path_segments(parser, segments)
            }
            Expression::Identifier { name } => {
                segments.push(*name);
                Some(())
            }
            Expression::QualifiedReference { path, .. } => {
                segments.extend_from_slice(&path.segments);
                Some(())
            }
            Expression::Member {
                left,
                name: Some(name),
                ..
            } => {
                let left = parser.tree.get(*left);
                left.collect_path_segments(parser, segments)?;
                segments.push(*name);
                Some(())
            }
            _ => None,
        }
    }
}

impl ExpressionPathLike for AssignPattern {
    fn collect_path_segments(&self, parser: &Parser, segments: &mut Vec<StringId>) -> Option<()> {
        match self {
            AssignPattern::Expression { value } => {
                let expression = parser.tree.get(*value);
                expression.collect_path_segments(parser, segments)
            }
            AssignPattern::Assign { pattern, .. } => {
                let pattern = parser.tree.get(*pattern);
                pattern.collect_path_segments(parser, segments)
            }
            AssignPattern::Sequence { .. } | AssignPattern::Object { .. } => None,
        }
    }
}

impl ExpressionPathLike for TypeExpression {
    fn collect_path_segments(&self, parser: &Parser, segments: &mut Vec<StringId>) -> Option<()> {
        match self {
            TypeExpression::Parenthesized { expression } => {
                let expression = parser.tree.get(*expression);
                expression.collect_path_segments(parser, segments)
            }
            TypeExpression::Reference { path, .. } => {
                segments.extend_from_slice(&path.segments);
                Some(())
            }
            _ => None,
        }
    }
}

/// Collect one value-space path string from an expression.
pub(crate) fn value_expression_path_string(
    parser: &Parser,
    expression: &Expression,
) -> Option<String> {
    let mut segments = Vec::new();
    collect_value_expression_path_segments(parser, expression, &mut segments)?;

    Some(
        segments
            .into_iter()
            .map(|segment| parser.strings.get(segment).to_string())
            .collect::<Vec<_>>()
            .join("."),
    )
}

/// Collect one value-space path as path segments.
fn collect_value_expression_path_segments(
    parser: &Parser,
    expression: &Expression,
    segments: &mut Vec<StringId>,
) -> Option<()> {
    match expression {
        Expression::Parenthesized { expression } => {
            let expression = parser.tree.get(*expression);
            collect_value_expression_path_segments(parser, expression, segments)
        }
        Expression::Identifier { name } => {
            segments.push(*name);
            Some(())
        }
        Expression::Member {
            left,
            name: Some(name),
            ..
        } => {
            let left = parser.tree.get(*left);
            collect_value_expression_path_segments(parser, left, segments)?;
            segments.push(*name);
            Some(())
        }
        _ => None,
    }
}

/// Collect one qualified-reference path string from an expression.
pub(crate) fn qualified_reference_path_string(
    parser: &Parser,
    expression: &impl ExpressionPathLike,
) -> Option<String> {
    let mut segments = Vec::new();
    expression.collect_path_segments(parser, &mut segments)?;

    Some(
        segments
            .into_iter()
            .map(|segment| parser.strings.get(segment).to_string())
            .collect::<Vec<_>>()
            .join("."),
    )
}

/// Normalize one comment payload for test assertions.
pub(crate) fn normalized_comment_payload(source: &str) -> std::borrow::Cow<'_, str> {
    normalize_comment_payload(source)
}

/// Assert one path-like expression directly against an expected path string.
///
/// This is a convenience helper for tests that only care about the visible
/// path spelling.
/// Use the stricter helpers when the parsed DIR shape matters.
#[macro_export]
macro_rules! assert_expression_path {
    ($parser:expr, $expr:expr, $expected:expr) => {{
        let got = $crate::tests::expression_path_string(&$parser, $expr);
        match got {
            Some(got) => assert_eq!(got, $expected, "expected path"),
            None => panic!("expected reference-like expression, got {:?}", $expr),
        };
    }};
}

/// Assert one value-space path expression directly against an expected path string.
#[macro_export]
macro_rules! assert_value_expression_path {
    ($parser:expr, $expr:expr, $expected:expr) => {{
        let got = $crate::tests::value_expression_path_string(&$parser, $expr);
        match got {
            Some(got) => assert_eq!(got, $expected, "expected value path"),
            None => panic!("expected value path expression, got {:?}", $expr),
        };
    }};
}

/// Assert one qualified-reference expression directly against an expected path string.
#[macro_export]
macro_rules! assert_qualified_reference_path {
    ($parser:expr, $expr:expr, $expected:expr) => {{
        let got = $crate::tests::qualified_reference_path_string(&$parser, $expr);
        match got {
            Some(got) => assert_eq!(got, $expected, "expected qualified reference"),
            None => panic!("expected qualified reference expression, got {:?}", $expr),
        };
    }};
}

/// Assert one comment trivia entry by kind and normalized source payload.
#[macro_export]
macro_rules! assert_comment {
    ($parser:expr, $index:expr, $expected_kind:expr, $expected_text:expr) => {{
        let comment = &$parser.tree.comments()[$index];
        assert_eq!(comment.kind, $expected_kind, "expected comment kind");

        let source = $parser.file.span_str(comment.span);
        let got = $crate::tests::normalized_comment_payload(source);
        assert_eq!(got.as_ref(), $expected_text, "expected comment text");
    }};
}
