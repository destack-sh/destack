pub use destack_unicode::UNICODE_VERSION;

/// A parsed Token.
/// It doesn't contain information about data that has been parsed,
/// only the type of the token and its size.
#[derive(Debug, PartialEq, Eq)]
pub struct Token {
    pub r#type: TokenType,
    pub len: u32,
}

impl Token {
    pub(crate) fn new(r#type: TokenType, len: u32) -> Token {
        Token { r#type, len }
    }
}

/// Enum representing common lexeme types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenType {
    /// A line comment, e.g. `// comment`.
    LineComment { doc_style: Option<DocPosition> },

    /// Any whitespace character sequence.
    Whitespace,

    /// An identifier or keyword, e.g. `identifier` or `continue`.
    Identifier,

    /// An identifier that is invalid because it contains emoji.
    InvalidIdentifier,

    /// Raw identifier, e.g. "r#identifier".
    RawIdentifier,

    /// An unknown literal prefix, like `foo#`, `foo'`, `foo"`.
    /// Excludes literal prefixes that contain emoji, which are considered "invalid".
    UnknownPrefix,

    /// Literals, e.g. `12u8`, `1.0e-40`, `b"123"`.
    /// NOTE: `_` is an invalid suffix, but may be present here on string and float literals.
    Literal {
        kind: LiteralTokenType,
        suffix_start: u32,
    },

    /// `;`
    Semi,
    /// `,`
    Comma,
    /// `.`
    Dot,
    /// `..`
    DotDot,
    /// `...`
    DotDotDot,
    /// `(`
    OpenParenthesis,
    /// `)`
    CloseParenthesis,
    /// `{`
    OpenBrace,
    /// `}`
    CloseBrace,
    /// `[`
    OpenBracket,
    /// `]`
    CloseBracket,
    /// `@`
    At,
    /// `#`
    Pound,
    /// `~`
    Tilde,
    /// `?`
    Question,
    /// `:`
    Colon,
    /// `::`
    DoubleColon,
    /// `$`
    Dollar,
    /// `=`
    Equals,
    /// `=>`
    FatArrow,
    /// `!`
    Bang,
    /// `<`
    LessThan,
    /// `>`
    GreaterThan,
    /// `-`
    Minus,
    /// `->`
    ThinArrow,
    /// `&`
    And,
    /// `|`
    Or,
    /// `+`
    Plus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `^`
    Caret,
    /// `%`
    Percent,
    /// Unknown token, not expected by the lexer, e.g. "№"
    Unknown,
    /// End of input.
    Eof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocPosition {
    Outer,
    Inner,
}

/// Literal Token
///
/// NOTE: The suffix is *not* considered when deciding the `LiteralType` in
/// this type. This means that float literals like `1f32` are classified by this
/// type as `Int`. (Compare against `destackc_ast::token::LitKind` and
/// `destackc_ast::ast::LitKind`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LiteralTokenType {
    /// `12_u8`, `0o100`, `0b120i99`, `1f32`.
    Integer { base: NumberBase, empty_int: bool },
    /// `12.34f32`, `1e3`, but not `1f32`.
    Float {
        base: NumberBase,
        empty_exponent: bool,
    },
    /// `'a'`, `'\\'`, `'''`, `';`
    Character { terminated: bool },
    /// `b'a'`, `b'\\'`, `b'''`, `b';`
    Byte { terminated: bool },
    /// `"abc"`, `"abc`
    String { terminated: bool },
    /// `b"abc"`, `b"abc`
    ByteString { terminated: bool },
    /// `r"abc"`, `r#"abc"#`, `r####"ab"###"c"####`, `r#"a`.
    RawString { n_hashes: Option<u8> },
    /// `br"abc"`, `br#"abc"#`, `br####"ab"###"c"####`, `br#"a`.
    RawByteString { n_hashes: Option<u8> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RawStringError {
    /// Non `#` characters exist between `r` and `"`, e.g. `r##~"abcde"##`
    InvalidStarter { bad_char: char },
    /// The string was not terminated, e.g. `r###"abcde"##`.
    /// `possible_terminator_offset` is the number of characters after `r` or
    /// `br` where they may have intended to terminate it.
    NoTerminator {
        expected: u32,
        found: u32,
        possible_terminator_offset: Option<u32>,
    },
    /// More than 255 `#`s exist.
    TooManyDelimiters { found: u32 },
}

/// Numeric literal base (according to its prefix).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NumberBase {
    /// Number starting with `0b`.
    Binary = 2,
    /// Number starting with `0o`.
    Octal = 8,
    /// Number doesn't contain a prefix.
    Decimal = 10,
    /// Number starting with `0x`.
    Hexadecimal = 16,
}
