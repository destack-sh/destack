use std::str::FromStr;

use dyst_source::Source;
use dyst_token::{RawLiteralType, TokenSpan, TokenType};

use crate::{
    Keyword, Node, NodeId, NodeTree, NodeVisitor, walk_argument, walk_enum_field, walk_parameter, walk_pattern_field, walk_struct_field, walk_type_literal, walk_union_field
};

/// The semantic type of a Span or Token.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum SemanticType {
    // lexical
    Whitespace,
    Identifier,
    Keyword,
    LiteralNumbery,
    LiteralStringy,
    Parenthesis,
    Symbol,
    Operator,
    Doc,
    Comment,
    // semantic
    Modifier,
    Macro,
    Type,
    Function,
    Parameter,
    Argument,
    Variable,
}

impl SemanticType {
    /// Map TokenType to *lexical* SemanticType.
    pub fn from_token(source: &Source, token: TokenSpan) -> Self {
        match token.token.r#type {
            // whitespace
            TokenType::Newline | TokenType::Whitespace | TokenType::Unknown | TokenType::End => {
                SemanticType::Whitespace
            }

            // annotations
            TokenType::LineComment | TokenType::BlockComment => SemanticType::Comment,
            TokenType::DocLineComment | TokenType::DocBlockComment => SemanticType::Doc,

            // identifiers / keywords
            TokenType::Identifier
            | TokenType::InvalidIdentifier
            | TokenType::UnknownLiteralPrefix => {
                if Keyword::from_str(source.get_span_str(token.span)).is_ok() {
                    SemanticType::Keyword
                } else {
                    SemanticType::Identifier
                }
            }

            // literals
            TokenType::Literal => {
                if let Some(literal) = token.token.body {
                    match literal {
                        RawLiteralType::Boolean { value: _ } => SemanticType::LiteralNumbery,
                        RawLiteralType::Int {
                            base: _,
                            is_empty: _,
                        } => SemanticType::LiteralNumbery,
                        RawLiteralType::Float {
                            base: _,
                            is_empty_exponent: _,
                        } => SemanticType::LiteralNumbery,
                        RawLiteralType::Character { is_terminated: _ } => {
                            SemanticType::LiteralStringy
                        }
                        RawLiteralType::Byte { is_terminated: _ } => SemanticType::LiteralStringy,
                        RawLiteralType::String { is_terminated: _ } => SemanticType::LiteralStringy,
                        RawLiteralType::ByteString { is_terminated: _ } => {
                            SemanticType::LiteralStringy
                        }
                        RawLiteralType::RawString { hashes: _ } => SemanticType::LiteralStringy,
                        RawLiteralType::RawByteString { hashes: _ } => SemanticType::LiteralStringy,
                    }
                } else {
                    SemanticType::LiteralStringy
                }
            }

            // symbols
            TokenType::Wildcard
            | TokenType::Colon
            | TokenType::Semicolon
            | TokenType::Comma
            | TokenType::Dot
            | TokenType::Range
            | TokenType::RangeWide
            | TokenType::FatArrow
            | TokenType::ThinArrow
            | TokenType::At
            | TokenType::Tag
            | TokenType::ElementwiseNot
            | TokenType::Maybe
            | TokenType::Coalesce
            | TokenType::Virtual
            | TokenType::Not => SemanticType::Operator,

            // parentheses
            TokenType::OpenParenthesis
            | TokenType::CloseParenthesis
            | TokenType::OpenBrace
            | TokenType::CloseBrace
            | TokenType::OpenBracket
            | TokenType::CloseBracket => SemanticType::Operator,

            // arithmetic operators
            TokenType::Multiply
            | TokenType::WrappingMultiply
            | TokenType::SaturatingMultiply
            | TokenType::Divide
            | TokenType::Remainder
            | TokenType::Add
            | TokenType::WrappingAdd
            | TokenType::SaturatingAdd
            | TokenType::Subtract
            | TokenType::WrappingSubtract
            | TokenType::SaturatingSubtract
            | TokenType::ShiftLeft
            | TokenType::SaturatingShiftLeft => SemanticType::Operator,

            // elementwise operators
            TokenType::ElementwiseAnd | TokenType::ElementwiseXor | TokenType::ElementwiseOr => {
                SemanticType::Operator
            }

            // comparison operators
            TokenType::Equal
            | TokenType::NotEqual
            | TokenType::LessThan
            | TokenType::LessThanOrEqual
            | TokenType::GreaterThan
            | TokenType::GreaterThanOrEqual => SemanticType::Operator,

            // logical operators
            TokenType::LogicalAnd | TokenType::LogicalOr => SemanticType::Operator,

            // assignment operators
            TokenType::Assign
            | TokenType::MultiplyAssign
            | TokenType::WrappingMultiplyAssign
            | TokenType::SaturatingMultiplyAssign
            | TokenType::DivideAssign
            | TokenType::RemainderAssign
            | TokenType::AddAssign
            | TokenType::WrappingAddAssign
            | TokenType::SaturatingAddAssign
            | TokenType::SubtractAssign
            | TokenType::WrappingSubtractAssign
            | TokenType::SaturatingSubtractAssign
            | TokenType::ShiftLeftAssign
            | TokenType::SaturatingShiftLeftAssign
            | TokenType::ShiftRightAssign
            | TokenType::ElementwiseAndAssign
            | TokenType::ElementwiseXorAssign
            | TokenType::ElementwiseOrAssign
            | TokenType::LogicalAndAssign
            | TokenType::LogicalOrAssign => SemanticType::Operator,
        }
    }
}

/// An index for SemanticTokenSpans.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticTokenIndex<'a> {
    /// The tokens to index.
    pub tokens: &'a Vec<TokenSpan>,
    /// The semantic types for the tokens (same length as tokens).
    pub semantic_types: Vec<SemanticType>,
}

impl<'a> SemanticTokenIndex<'a> {
    /// Create a new SemanticTokenIndex from a list of tokens.
    /// Immediately walks tokens and initialies to lexical semantic types.
    pub fn from_tokens(source: &Source, tokens: &'a Vec<TokenSpan>) -> Self {
        // start with lexical types
        let mut semantic_types = vec![SemanticType::Keyword; tokens.len()];
        for (i, token) in tokens.iter().enumerate() {
            semantic_types[i] = SemanticType::from_token(source, *token);
        }

        Self {
            tokens,
            semantic_types,
        }
    }

    /// Set the semantic type for a span.
    pub(crate) fn set_semantic_span<T: Node>(
        &mut self,
        tree: &NodeTree,
        id: NodeId<T>,
        semantic_type: SemanticType,
    ) {
        let span = tree.get_span(id);
        for (i, token) in self.tokens.iter().enumerate() {
            if token.span.contains(span.start) || span.contains(span.end) {
                self.semantic_types[i] = semantic_type;
            }
        }
    }
}

impl<'a> NodeVisitor for SemanticTokenIndex<'a> {
    // ------------------------------------------------------------
    // Types
    // ------------------------------------------------------------

    fn visit_type_literal(
        &mut self,
        tree: &NodeTree,
        id: NodeId<super::TypeLiteral>,
        type_literal: &super::TypeLiteral,
    ) {
        walk_type_literal(self, tree, id, type_literal);
        self.set_semantic_span(tree, id, SemanticType::Type);
    }

    fn visit_struct_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<super::StructField>,
        struct_field: &super::StructField,
    ) {
        walk_struct_field(self, tree, id, struct_field);
        self.set_semantic_span(tree, id, SemanticType::Variable);
    }

    fn visit_enum_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<super::EnumField>,
        enum_field: &super::EnumField,
    ) {
        walk_enum_field(self, tree, id, enum_field);
        self.set_semantic_span(tree, id, SemanticType::Variable);
    }

    fn visit_union_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<super::UnionField>,
        union_field: &super::UnionField,
    ) {
        walk_union_field(self, tree, id, union_field);
        self.set_semantic_span(tree, id, SemanticType::Variable);
    }

    // ------------------------------------------------------------
    // Bindings
    // ------------------------------------------------------------

    fn visit_parameter(
        &mut self,
        tree: &NodeTree,
        id: NodeId<super::Parameter>,
        parameter: &super::Parameter,
    ) {
        walk_parameter(self, tree, id, parameter);
        self.set_semantic_span(tree, id, SemanticType::Parameter);
    }

    fn visit_argument(
        &mut self,
        tree: &NodeTree,
        id: NodeId<super::Argument>,
        argument: &super::Argument,
    ) {
        walk_argument(self, tree, id, argument);
        self.set_semantic_span(tree, id, SemanticType::Argument);
    }

    // ------------------------------------------------------------
    // Matching
    // ------------------------------------------------------------

    fn visit_pattern_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<super::PatternField>,
        pattern_field: &super::PatternField,
    ) {
        walk_pattern_field(self, tree, id, pattern_field);
        self.set_semantic_span(tree, id, SemanticType::Variable);
    }
}
