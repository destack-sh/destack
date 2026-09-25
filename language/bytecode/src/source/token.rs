use serde::{Deserialize, Serialize};
use tspp_serde::Reflect;
use tspp_source::Span;

/// One lexical bytecode token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Token {
    /// The token category.
    pub ty: TokenType,
    /// The exact source span.
    pub span: Span,
}

impl Token {
    /// Create one token.
    pub const fn new(ty: TokenType, span: Span) -> Self {
        Self { ty, span }
    }

    /// Return whether this token is trivia.
    pub const fn is_trivia(self) -> bool {
        matches!(
            self.ty,
            TokenType::Whitespace | TokenType::Newline | TokenType::Comment
        )
    }
}

/// Bytecode token category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum TokenType {
    /// Identifier or dotted operation name.
    Identifier,
    /// Integer literal.
    Integer,
    /// Floating-point literal.
    Float,
    /// Non-newline whitespace.
    Whitespace,
    /// Newline sequence.
    Newline,
    /// Line comment.
    Comment,
    /// End of source.
    End,
    /// Unknown token.
    Unknown,
    /// `(`.
    OpenParenthesis,
    /// `)`.
    CloseParenthesis,
    /// `{`.
    OpenBrace,
    /// `}`.
    CloseBrace,
    /// `[`.
    OpenBracket,
    /// `]`.
    CloseBracket,
    /// `<`.
    LessThan,
    /// `>`.
    GreaterThan,
    /// `:`.
    Colon,
    /// `;`.
    Semicolon,
    /// `,`.
    Comma,
    /// `|`.
    Pipe,
    /// `@`.
    At,
    /// `*`.
    Star,
    /// `=`.
    Equal,
    /// `->`.
    Arrow,
    /// `=>`.
    FatArrow,
}
