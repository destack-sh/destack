use std::str::FromStr;

use crate::{
    Argument, Definition, EnumField, Expression, Keyword, LiteralType, Node, NodeId, NodeTree,
    NodeVisitor, Parameter, PatternField, ScalarLiteral, TokenSpan, TokenType, UnionField,
    VariantField, walk_argument, walk_definition, walk_enum_field, walk_expression, walk_parameter,
    walk_pattern_field, walk_union_field, walk_variant_field,
};
use dyst_source::Source;

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
    pub fn from_token(source: &Source, token: &TokenSpan) -> Self {
        match token.token.ty {
            // --------------------------------------------------
            // Structural
            // --------------------------------------------------
            TokenType::Newline
            | TokenType::Whitespace
            | TokenType::Unknown
            | TokenType::End => SemanticType::Whitespace,

            // --------------------------------------------------
            // Annotations
            // --------------------------------------------------
            TokenType::LineComment
            | TokenType::BlockComment => SemanticType::Comment,
            TokenType::DocLineComment
            | TokenType::DocBlockComment => SemanticType::Doc,
            // (tags are not parsed as tokens)

            // --------------------------------------------------
            // Identifiers / Literals
            // --------------------------------------------------
            TokenType::Identifier
            | TokenType::InvalidIdentifier
            | TokenType::UnknownLiteralPrefix => {
                if Keyword::from_str(source.get_span_str(token.span)).is_ok() {
                    SemanticType::Keyword
                } else {
                    SemanticType::Identifier
                }
            }
            TokenType::Literal => {
                if let Some(literal) = token.token.literal {
                    match literal {
                        LiteralType::Boolean { .. } => SemanticType::LiteralNumbery,
                        LiteralType::Int { .. } => SemanticType::LiteralNumbery,
                        LiteralType::Float { .. } => SemanticType::LiteralNumbery,
                        LiteralType::Character { .. } => SemanticType::LiteralStringy,
                        LiteralType::Byte { .. } => SemanticType::LiteralStringy,
                        LiteralType::String { .. } => SemanticType::LiteralStringy,
                        LiteralType::ByteString { .. } => SemanticType::LiteralStringy,
                        LiteralType::RegexString { .. } => SemanticType::LiteralStringy,
                        LiteralType::RawString { .. } => SemanticType::LiteralStringy,
                        LiteralType::RawByteString { .. } => SemanticType::LiteralStringy,
                    }
                } else {
                    SemanticType::LiteralStringy
                }
            }
            TokenType::TemplateStringStart
            | TokenType::TemplateStringMiddle
            | TokenType::TemplateStringEnd
            | TokenType::TemplateString => SemanticType::LiteralStringy,

            // --------------------------------------------------
            // Symbols
            // --------------------------------------------------
            TokenType::Wildcard => SemanticType::Operator,
            TokenType::Colon => SemanticType::Operator,
            TokenType::Semicolon => SemanticType::Operator,
            TokenType::Comma => SemanticType::Operator,
            TokenType::Dot => SemanticType::Operator,
            TokenType::Range => SemanticType::Operator,
            TokenType::RangeWide => SemanticType::Operator,
            TokenType::Arrow => SemanticType::Operator,
            TokenType::ArrowWide => SemanticType::Operator,
            TokenType::At => SemanticType::Operator,
            TokenType::Tag => SemanticType::Operator,

            // --------------------------------------------------
            // Elementwise / Logical / Dynamic prefixes
            // --------------------------------------------------
            TokenType::ElementwiseNot => SemanticType::Operator,
            TokenType::Maybe => SemanticType::Operator,
            TokenType::Coalesce => SemanticType::Operator,
            TokenType::Dynamic => SemanticType::Operator,
            TokenType::Not => SemanticType::Operator,

            // --------------------------------------------------
            // Parentheses
            // --------------------------------------------------
            TokenType::OpenParenthesis
            | TokenType::CloseParenthesis
            | TokenType::OpenBrace
            | TokenType::CloseBrace
            | TokenType::OpenBracket
            | TokenType::CloseBracket => SemanticType::Operator,

            // --------------------------------------------------
            // Multiplication
            // --------------------------------------------------
            TokenType::Multiply
            | TokenType::WrappingMultiply
            | TokenType::SaturatingMultiply
            | TokenType::Divide
            | TokenType::Remainder

            // --------------------------------------------------
            // Addition
            // --------------------------------------------------
            | TokenType::Add
            | TokenType::WrappingAdd
            | TokenType::SaturatingAdd
            | TokenType::Subtract
            | TokenType::WrappingSubtract
            | TokenType::SaturatingSubtract
            | TokenType::Increment
            | TokenType::Decrement

            // --------------------------------------------------
            // Shift
            // --------------------------------------------------
            | TokenType::ShiftLeft
            | TokenType::SaturatingShiftLeft

            // --------------------------------------------------
            // Elementwise
            // --------------------------------------------------
            | TokenType::ElementwiseAnd
            | TokenType::ElementwiseXor
            | TokenType::ElementwiseOr

            // --------------------------------------------------
            // Comparison
            // --------------------------------------------------
            | TokenType::Equal
            | TokenType::EqualWide
            | TokenType::NotEqual
            | TokenType::NotEqualWide
            | TokenType::LessThan
            | TokenType::LessThanOrEqual
            | TokenType::GreaterThan
            | TokenType::GreaterThanOrEqual

            // --------------------------------------------------
            // Logical
            // --------------------------------------------------
            | TokenType::LogicalAnd
            | TokenType::LogicalOr => SemanticType::Operator,

            // --------------------------------------------------
            // Assignment
            // --------------------------------------------------
            TokenType::Assign

            // --------------------------------------------------
            // Assignment Multiplication
            // --------------------------------------------------
            | TokenType::MultiplyAssign
            | TokenType::WrappingMultiplyAssign
            | TokenType::SaturatingMultiplyAssign
            | TokenType::DivideAssign
            | TokenType::RemainderAssign

            // --------------------------------------------------
            // Assignment Addition
            // --------------------------------------------------
            | TokenType::AddAssign
            | TokenType::WrappingAddAssign
            | TokenType::SaturatingAddAssign
            | TokenType::SubtractAssign
            | TokenType::WrappingSubtractAssign
            | TokenType::SaturatingSubtractAssign

            // --------------------------------------------------
            // Assignment Shift
            // --------------------------------------------------
            | TokenType::ShiftLeftAssign
            | TokenType::SaturatingShiftLeftAssign
            | TokenType::ShiftRightAssign

            // --------------------------------------------------
            // Assignment Elementwise
            // --------------------------------------------------
            | TokenType::ElementwiseAndAssign
            | TokenType::ElementwiseXorAssign
            | TokenType::ElementwiseOrAssign

            // --------------------------------------------------
            // Assignment Logical
            // --------------------------------------------------
            | TokenType::LogicalAndAssign
            | TokenType::LogicalOrAssign
            | TokenType::CoalesceAssign => SemanticType::Operator,
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
        // initialize with lexical types
        let mut semantic_types = vec![SemanticType::Keyword; tokens.len()];
        for (i, token) in tokens.iter().enumerate() {
            semantic_types[i] = SemanticType::from_token(source, token);
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
    fn visit_expression(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Expression>,
        expression: &Expression,
    ) {
        match expression {
            Expression::ScalarLiteral(
                ScalarLiteral::Character(_)
                | ScalarLiteral::String(_)
                | ScalarLiteral::ByteString(_)
                | ScalarLiteral::RegexString { .. },
            ) => {
                self.set_semantic_span(tree, id, SemanticType::LiteralStringy);
            }
            Expression::TypeLiteral(_) => {
                self.set_semantic_span(tree, id, SemanticType::Type);
            }
            _ => {}
        }
        walk_expression(self, tree, id, expression);
    }

    // ------------------------------------------------------------
    // Types
    // ------------------------------------------------------------

    fn visit_definition(
        &mut self,
        tree: &NodeTree,
        id: NodeId<Definition>,
        definition: &Definition,
    ) {
        walk_definition(self, tree, id, definition);
    }

    fn visit_variant_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<VariantField>,
        variant_field: &VariantField,
    ) {
        walk_variant_field(self, tree, id, variant_field);
        self.set_semantic_span(tree, id, SemanticType::Variable);
        self.set_semantic_span(tree, variant_field.ty(), SemanticType::Type);
    }

    fn visit_enum_field(&mut self, tree: &NodeTree, id: NodeId<EnumField>, enum_field: &EnumField) {
        walk_enum_field(self, tree, id, enum_field);
        self.set_semantic_span(tree, id, SemanticType::Variable);
    }

    fn visit_union_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<UnionField>,
        union_field: &UnionField,
    ) {
        walk_union_field(self, tree, id, union_field);
        self.set_semantic_span(tree, id, SemanticType::Variable);
    }

    // ------------------------------------------------------------
    // Bindings
    // ------------------------------------------------------------

    fn visit_parameter(&mut self, tree: &NodeTree, id: NodeId<Parameter>, parameter: &Parameter) {
        walk_parameter(self, tree, id, parameter);
        self.set_semantic_span(tree, id, SemanticType::Parameter);
        match parameter {
            Parameter::Named {
                name: _,
                ty,
                default: _,
            } => {
                if let Some(ty) = ty {
                    self.set_semantic_span(tree, *ty, SemanticType::Type);
                }
            }
            Parameter::Pattern {
                pattern: _,
                ty,
                default: _,
            } => {
                if let Some(ty) = ty {
                    self.set_semantic_span(tree, *ty, SemanticType::Type);
                }
            }
            Parameter::Variadic { name: _, ty } => {
                if let Some(ty) = ty {
                    self.set_semantic_span(tree, *ty, SemanticType::Type);
                }
            }
        }
    }

    fn visit_argument(&mut self, tree: &NodeTree, id: NodeId<Argument>, argument: &Argument) {
        walk_argument(self, tree, id, argument);
        self.set_semantic_span(tree, id, SemanticType::Argument);
    }

    // ------------------------------------------------------------
    // Matching
    // ------------------------------------------------------------

    fn visit_pattern_field(
        &mut self,
        tree: &NodeTree,
        id: NodeId<PatternField>,
        pattern_field: &PatternField,
    ) {
        walk_pattern_field(self, tree, id, pattern_field);
        self.set_semantic_span(tree, id, SemanticType::Variable);
    }
}
