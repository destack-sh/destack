use std::sync::Arc;
use tspp_dir::{
    AssignPattern, Block, Expression, LocalNodeId, Node, NodeType, NumberBase, TokenLiteral,
    TokenType, Tree, TypeExpression,
};

use tspp_core::StringId;
use tspp_source::{File, FileId, FileType, LanguageType, ModuleId, PackageId, Uri};

use crate::{CommentRetention, ParseOptions, Parser, SourceForm};

/// A test wrapper for Parser.
#[derive(Debug)]
pub(crate) struct TestParser {
    /// The source file under test.
    pub file: Arc<File>,
    /// The source language under test.
    pub language: LanguageType,
}

impl TestParser {
    /// Create a parser test for `.tspp` source.
    pub(crate) fn new(input: &str) -> Self {
        Self::with_language(input, LanguageType::Tspp)
    }

    /// Create a parser test for `.d.tspp` source.
    pub(crate) fn declaration(input: &str) -> Self {
        Self::with_language(input, LanguageType::TsppDeclaration)
    }

    /// Create a parser test for one source form.
    fn with_language(input: &str, language: LanguageType) -> Self {
        let file_id = FileId::new(0);
        let file = File::from_text(
            file_id,
            "<string>".to_string(),
            Uri::from_string("<string>"),
            None,
            FileType::from(language),
            input.to_string(),
        )
        .expect("test source should load");
        Self {
            file: Arc::new(file),
            language,
        }
    }

    /// Create a parser for this test.
    pub(crate) fn prepare(&self) -> Parser {
        self.prepare_with_comment_retention(CommentRetention::All)
    }

    /// Create a parser accepting structural Pattern placeholders.
    pub(crate) fn prepare_pattern(&self) -> Parser {
        self.prepare_with_options(ParseOptions {
            form: SourceForm::Pattern,
            comment_retention: CommentRetention::All,
        })
    }

    /// Create a parser with explicit comment retention for this test.
    pub(crate) fn prepare_with_comment_retention(
        &self,
        comment_retention: CommentRetention,
    ) -> Parser {
        self.prepare_with_options(ParseOptions {
            comment_retention,
            ..ParseOptions::default()
        })
    }

    /// Create a parser with explicit parse options for this test.
    fn prepare_with_options(&self, options: ParseOptions) -> Parser {
        let module_id = ModuleId::new(PackageId::new(0), self.file.id.0);
        let tree = Tree::new(module_id);

        Parser::new(self.file.clone(), self.language, tree, options)
    }

    /// Parse one complete source file without parser errors.
    pub(crate) fn parse(&self) -> (Parser, Vec<LocalNodeId<Expression>>) {
        let mut parser = self.prepare();
        let roots = parser.parse_in_place();

        self.assert_no_errors(&parser);

        (parser, roots)
    }

    /// Assert parser errors by node type, actual token, expected token, and source text.
    pub(crate) fn assert_errors(
        &self,
        parser: &Parser,
        expected_errors: &[(Option<NodeType>, Option<TokenType>, Option<TokenType>, &str)],
    ) {
        let actual_errors: Vec<_> = parser
            .errors
            .iter()
            .map(|error| {
                (
                    error.expected_node(),
                    error.actual_token(),
                    error.expected_token(),
                    parser.range_str(error.range()).to_owned(),
                )
            })
            .collect();
        let expected_errors: Vec<_> = expected_errors
            .iter()
            .map(|(node_type, actual, expected, text)| {
                (*node_type, *actual, *expected, (*text).to_owned())
            })
            .collect();

        assert_eq!(actual_errors, expected_errors);
    }

    /// Assert that the parser produced no diagnostics.
    pub(crate) fn assert_no_errors(&self, parser: &Parser) {
        assert!(parser.errors.is_empty(), "{:#?}", parser.errors);
    }

    /// Return normalized documentation attached to one parsed node.
    pub(crate) fn documentation<'a, T: Node>(
        &self,
        parser: &'a Parser,
        node_id: LocalNodeId<T>,
    ) -> Option<&'a str> {
        parser
            .tree
            .get_documentation(node_id.id)
            .map(|documentation| parser.strings.get(documentation.markdown))
    }
}

/// Return the complete semantic stream before grammar consumption.
#[test]
fn test_take_tokens_before_parse() {
    let test = TestParser::new("const value = 1;");
    let mut parser = test.prepare();
    let tokens = parser.take_tokens();
    let token_types: Vec<_> = tokens.iter().map(|token| token.ty()).collect();

    assert_eq!(
        token_types,
        vec![
            TokenType::Identifier,
            TokenType::Identifier,
            TokenType::Assign,
            TokenType::Literal,
            TokenType::Semicolon,
            TokenType::End,
        ]
    );
}

/// Materialize split compound tokens after parsing.
#[test]
fn test_take_tokens_materializes_splits() {
    let test = TestParser::new("const borrowed = &&value; type Nested = Box<Box<int>>;");
    let mut parser = test.prepare();
    parser.parse_in_place();
    let tokens = parser.take_tokens();
    let tokens: Vec<_> = tokens
        .iter()
        .map(|token| (token.ty(), parser.token_str(*token)))
        .collect();

    assert_eq!(
        tokens,
        vec![
            (TokenType::Identifier, "const"),
            (TokenType::Identifier, "borrowed"),
            (TokenType::Assign, "="),
            (TokenType::ElementwiseAnd, "&"),
            (TokenType::ElementwiseAnd, "&"),
            (TokenType::Identifier, "value"),
            (TokenType::Semicolon, ";"),
            (TokenType::Identifier, "type"),
            (TokenType::Identifier, "Nested"),
            (TokenType::Assign, "="),
            (TokenType::Identifier, "Box"),
            (TokenType::LessThan, "<"),
            (TokenType::Identifier, "Box"),
            (TokenType::LessThan, "<"),
            (TokenType::Identifier, "int"),
            (TokenType::GreaterThan, ">"),
            (TokenType::GreaterThan, ">"),
            (TokenType::Semicolon, ";"),
            (TokenType::End, ""),
        ]
    );
    test.assert_no_errors(&parser);
}

/// Materialize one contextually reclassified regex token after parsing.
#[test]
fn test_take_tokens_materializes_regex() {
    let test = TestParser::new("const pattern = /[/*]/g; /** next */ const value = 1;");
    let mut parser = test.prepare();
    parser.parse_in_place();
    let tokens = parser.take_tokens();
    let tokens: Vec<_> = tokens
        .iter()
        .map(|token| (token.ty(), token.literal(), parser.token_str(*token)))
        .collect();

    assert_eq!(
        tokens,
        vec![
            (TokenType::Identifier, None, "const"),
            (TokenType::Identifier, None, "pattern"),
            (TokenType::Assign, None, "="),
            (
                TokenType::Literal,
                Some(TokenLiteral::RegexString { has_flags: true }),
                "/[/*]/g",
            ),
            (TokenType::Semicolon, None, ";"),
            (TokenType::Identifier, None, "const"),
            (TokenType::Identifier, None, "value"),
            (TokenType::Assign, None, "="),
            (
                TokenType::Literal,
                Some(TokenLiteral::Int {
                    base: NumberBase::Decimal,
                    is_empty: false,
                    is_bigint: false,
                }),
                "1",
            ),
            (TokenType::Semicolon, None, ";"),
            (TokenType::End, None, ""),
        ]
    );
    let comments: Vec<_> = parser
        .comments()
        .iter()
        .map(|comment| comment.text(parser.file.text()).into_owned())
        .collect();
    assert_eq!(comments, vec![" next"]);
    test.assert_no_errors(&parser);
}

/// Materialize contextually tokenized tree names and text after parsing.
#[test]
fn test_take_tokens_materializes_tree() {
    let test =
        TestParser::new("const tree = <panel-name title={value}>hello /* world */</panel-name>;");
    let mut parser = test.prepare();
    parser.parse_in_place();
    let tokens = parser.take_tokens();
    let tokens: Vec<_> = tokens
        .iter()
        .map(|token| (token.ty(), token.literal(), parser.token_str(*token)))
        .collect();

    assert_eq!(
        tokens,
        vec![
            (TokenType::Identifier, None, "const"),
            (TokenType::Identifier, None, "tree"),
            (TokenType::Assign, None, "="),
            (TokenType::LessThan, None, "<"),
            (TokenType::Identifier, None, "panel"),
            (TokenType::Subtract, None, "-"),
            (TokenType::Identifier, None, "name"),
            (TokenType::Identifier, None, "title"),
            (TokenType::Assign, None, "="),
            (TokenType::OpenBrace, None, "{"),
            (TokenType::Identifier, None, "value"),
            (TokenType::CloseBrace, None, "}"),
            (TokenType::GreaterThan, None, ">"),
            (
                TokenType::Literal,
                Some(TokenLiteral::TreeString),
                "hello /* world */"
            ),
            (TokenType::LessThan, None, "<"),
            (TokenType::Divide, None, "/"),
            (TokenType::Identifier, None, "panel"),
            (TokenType::Subtract, None, "-"),
            (TokenType::Identifier, None, "name"),
            (TokenType::GreaterThan, None, ">"),
            (TokenType::Semicolon, None, ";"),
            (TokenType::End, None, ""),
        ]
    );
    assert!(parser.comments().is_empty());
    test.assert_no_errors(&parser);
}

/// Return EOF for lookahead offsets beyond the physical end of source.
#[test]
fn test_lookahead_stays_at_end() {
    let test = TestParser::new("value");
    let parser = test.prepare();

    assert_eq!(parser.peek_token_type_at(1), TokenType::End);
    assert_eq!(parser.peek_token_type_at(2), TokenType::End);
    assert_eq!(parser.peek_token_type_at(8), TokenType::End);
    assert_eq!(parser.peek_token_type(), TokenType::Identifier);
}

/// Collect block expressions in source order for test assertions.
pub(crate) fn block_expression_ids(block: &Block) -> Vec<LocalNodeId<Expression>> {
    block.iter_expressions().collect()
}

/// Assert that one canonical node carries a written parentheses region.
#[macro_export]
macro_rules! assert_parenthesized {
    ($tree:expr, $id:expr) => {{
        let node_id = $id;
        let parentheses = $tree.get_side_range(
            node_id,
            tspp_source::NodeSpanType::Region(tspp_source::NodeSpanRegion::Parentheses),
        );
        assert!(parentheses.is_some(), "expected written parentheses");
    }};
    ($tree:expr, $id:expr, $binding:ident => $body:block) => {{
        let node_id = $id;
        let parentheses = $tree.get_side_range(
            node_id,
            tspp_source::NodeSpanType::Region(tspp_source::NodeSpanRegion::Parentheses),
        );
        assert!(parentheses.is_some(), "expected written parentheses");
        let $binding = &node_id;
        $body
    }};
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
        let maybe_id: Option<tspp_core::StringId> = ::core::convert::Into::into($id);
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
            Expression::Type { value } => {
                let expression = parser.tree.get(*value);
                expression.collect_path_segments(parser, segments)
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
            AssignPattern::Place { expression: value } => {
                let expression = parser.tree.get(*value);
                expression.collect_path_segments(parser, segments)
            }
            AssignPattern::Default { pattern, .. } => {
                let pattern = parser.tree.get(*pattern);
                pattern.collect_path_segments(parser, segments)
            }
            AssignPattern::Sequence { .. }
            | AssignPattern::Tuple { .. }
            | AssignPattern::Object { .. } => None,
        }
    }
}

impl ExpressionPathLike for TypeExpression {
    fn collect_path_segments(&self, _parser: &Parser, segments: &mut Vec<StringId>) -> Option<()> {
        match self {
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

/// Assert one comment trivia entry by kind and normalized source payload.
#[macro_export]
macro_rules! assert_comment {
    ($parser:expr, $index:expr, $expected_kind:expr, $expected_text:expr) => {{
        let comment = &$parser.comments()[$index];
        assert_eq!(comment.kind, $expected_kind, "expected comment kind");

        let got = comment.text($parser.file.text());
        assert_eq!(got.as_ref(), $expected_text, "expected comment text");
    }};
}
