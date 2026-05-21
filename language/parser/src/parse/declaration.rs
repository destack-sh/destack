use destack_dir::{ExportKind, Keyword, TokenType};
use destack_source::Span;

/// Tokens that can start a declaration binding pattern.
pub static DECLARATION_START_TOKENS: [TokenType; 6] = [
    TokenType::Literal,
    TokenType::Identifier,
    TokenType::OpenParenthesis,
    TokenType::OpenBrace,
    TokenType::OpenBracket,
    TokenType::LessThan,
];

/// Tokens that can start a value pattern.
pub static PATTERN_START_TOKENS: [TokenType; 6] = [
    TokenType::Identifier,
    TokenType::Literal,
    TokenType::ElementwiseAnd,
    TokenType::OpenParenthesis,
    TokenType::OpenBrace,
    TokenType::OpenBracket,
];

/// Parsed declaration prefix shared across declaration forms.
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub(crate) struct DeclarationHeader {
    /// The export kind for the declaration.
    pub export: Option<ExportKind>,
    /// Whether the declaration is ambient.
    pub is_ambient: bool,
    /// The explicit `declare` modifier span.
    pub declare_span: Option<Span>,
    /// Whether the declaration is abstract.
    pub is_abstract: bool,
    /// Whether the declaration is final.
    pub is_final: bool,
    /// Whether the declaration has shared placement.
    pub is_shared: bool,
}

impl DeclarationHeader {
    /// Return whether the header contains a parsed prefix modifier.
    pub(crate) fn has_modifier(self) -> bool {
        self.is_ambient
            || self.declare_span.is_some()
            || self.is_abstract
            || self.is_final
            || self.is_shared
    }
}

/// Return true when a keyword starts a declaration.
pub(crate) fn is_declaration_keyword(keyword: Keyword) -> bool {
    matches!(
        keyword,
        Keyword::Declare
            | Keyword::Abstract
            | Keyword::Final
            | Keyword::Struct
            | Keyword::Class
            | Keyword::Enum
            | Keyword::Function
            | Keyword::Extension
            | Keyword::Interface
            | Keyword::Type
            | Keyword::Newtype
            | Keyword::Const
            | Keyword::Readonly
            | Keyword::Let
            | Keyword::Using
            | Keyword::Override
            | Keyword::Public
            | Keyword::Protected
            | Keyword::Private
            | Keyword::Async
    )
}

/// Return true when a keyword can act as a type relation operator.
pub(crate) fn is_type_relation_keyword(keyword: Option<Keyword>) -> bool {
    matches!(
        keyword,
        Some(
            Keyword::As
                | Keyword::Satisfies
                | Keyword::Is
                | Keyword::InstanceOf
                | Keyword::In
                | Keyword::Extends
                | Keyword::Implements
        )
    )
}
