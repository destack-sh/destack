use tspp_dir::{ExportKind, Keyword, Mutability, TokenType, TypeKind};
use tspp_source::ByteRange;

/// Tokens that can start a declaration binding pattern.
pub const DECLARATION_START_TOKENS: [TokenType; 6] = [
    TokenType::Literal,
    TokenType::Identifier,
    TokenType::OpenParenthesis,
    TokenType::OpenBrace,
    TokenType::OpenBracket,
    TokenType::LessThan,
];

/// Tokens that can start a value pattern.
pub const PATTERN_START_TOKENS: [TokenType; 6] = [
    TokenType::Identifier,
    TokenType::Literal,
    TokenType::ElementwiseAnd,
    TokenType::OpenParenthesis,
    TokenType::OpenBrace,
    TokenType::OpenBracket,
];

/// One declaration header shared across declaration forms.
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub(crate) struct DeclarationHeader {
    /// The export kind for the declaration.
    pub(crate) export: Option<ExportKind>,
    /// Whether the declaration is ambient.
    pub(crate) is_ambient: bool,
    /// The explicit `declare` modifier range.
    pub(crate) declare_range: Option<ByteRange>,
    /// Whether the declaration is abstract.
    pub(crate) is_abstract: bool,
    /// Whether the declaration is final.
    pub(crate) is_final: bool,
    /// Whether the declaration is shared.
    pub(crate) is_shared: bool,
}

/// The declaration properties implied by one type keyword.
#[derive(Debug, Copy, Clone)]
pub(crate) struct TypeKeywordHeader {
    /// The type declaration kind implied by the keyword.
    pub(crate) kind: TypeKind,
    /// The alias mutability implied by the keyword.
    pub(crate) mutability: Option<Mutability>,
}

impl TypeKeywordHeader {
    /// Return the declaration properties implied by one type keyword.
    pub(crate) fn from_keyword(keyword: Keyword) -> Option<Self> {
        let kind = match keyword {
            Keyword::Type | Keyword::Readonly => TypeKind::Structural,
            Keyword::Newtype => TypeKind::Nominal,
            _ => return None,
        };
        let mutability = (keyword == Keyword::Readonly).then_some(Mutability::Immutable);

        Some(Self { kind, mutability })
    }
}
