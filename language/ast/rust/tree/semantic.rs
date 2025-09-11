use std::str::FromStr;

use dyst_language_source::Source;
use dyst_language_token::{RawLiteralType, TokenSpan, TokenType};

use crate::{Keyword, NodeTree, NodeVisitor, Type, walk_type};

/// The semantic type of a Span or Token.
#[derive(Debug, Clone, PartialEq)]
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
                        RawLiteralType::Void => SemanticType::LiteralNumbery,
                        RawLiteralType::Null => SemanticType::LiteralNumbery,
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
            | TokenType::Pound
            | TokenType::Empty
            | TokenType::EmptyWide
            | TokenType::Arrow
            | TokenType::BadArrow
            | TokenType::At
            | TokenType::BitwiseNot
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
            | TokenType::SaturatingShiftLeft
            | TokenType::ShiftRight => SemanticType::Operator,

            // bitwise operators
            TokenType::BitwiseAnd | TokenType::BitwiseXor | TokenType::BitwiseOr => {
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
            | TokenType::BitwiseAndAssign
            | TokenType::BitwiseXorAssign
            | TokenType::BitwiseOrAssign
            | TokenType::LogicalAndAssign
            | TokenType::LogicalOrAssign => SemanticType::Operator,
        }
    }
}

/// A builder for SemanticTokenSpans.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticTokenMap<'a> {
    pub tokens: &'a Vec<TokenSpan>,
    pub semantic_types: Vec<SemanticType>, // same length as tokens
}

impl<'a> SemanticTokenMap<'a> {
    pub fn from_tokens(source: &Source, tokens: &'a Vec<TokenSpan>) -> Self {
        // start from lexical types
        let mut semantic_types = vec![SemanticType::Keyword; tokens.len()];
        for (i, token) in tokens.iter().enumerate() {
            semantic_types[i] = SemanticType::from_token(source, *token);
        }
        Self {
            tokens,
            semantic_types,
        }
    }
}

// nocheckin: semantic tokens proper
impl<'a> NodeVisitor for SemanticTokenMap<'a> {
    fn visit_type(&mut self, tree: &NodeTree, type_node: &Type) {
        walk_type(self, tree, type_node);
    }
}
