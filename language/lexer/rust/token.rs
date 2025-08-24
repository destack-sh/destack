use std::fmt::Display;

pub use destack_library_unicode::UNICODE_VERSION;

// TODO: define Token/TokenType/... in Destack

/// A parsed Token.
/// It doesn't contain information about data that has been parsed,
/// only the type of the token and its size.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Token {
    /// The Token tag.
    pub r#type: TokenType,
    /// The length of the token in bytes.
    pub len: u32,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<Token {:?}, {}>", self.r#type, self.len)
    }
}

impl Token {
    pub(crate) fn new(r#type: TokenType, len: u32) -> Token {
        Token { r#type, len }
    }
}

/// Enum representing common lexeme types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenType {
    /// Any whitespace character sequence.
    Whitespace,
    /// A line comment, e.g. `// comment` `//// comment` `//////// comment`.
    LineComment,
    /// Unknown/Unexpected (e.g., '№')
    Unknown,
    /// End of sequence (e.g., end of source file)
    EndOfInput,

    /// A doc line comment with exactly three slashes, e.g. `/// doc comment`. or `///`
    DocComment,
    /// An identifier or keyword, e.g. `identifier` or `continue`.
    Identifier,
    /// An identifier that is invalid (e.g. because it contains emoji).
    InvalidIdentifier,
    /// An unknown literal prefix, like `foo#`, `foo'`, `foo"`.
    UnknownLiteralPrefix,
    /// Literals, e.g. `12u8`, `1.0e-40`, `b"123"`.
    Literal {
        r#type: LiteralTokenType,
        suffix_start: u32,
    },

    /// `:`
    Colon,
    /// `;`
    Semicolon,
    /// `,`
    Comma,
    /// `.`
    Dot,
    /// `..`
    Range,
    /// `...`
    Ellipsis,

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
    /// `$`
    Dollar,
    /// `!`
    Bang,
    /// `-`
    Subtract,
    /// '---',
    Empty,
    /// `&`
    BitwiseAnd,
    /// `&&`
    LogicalAnd,
    /// `|`
    BitwiseOr,
    /// `||`
    LogicalOr,
    /// `+`
    Add,
    /// `*`
    Multiply,
    /// `/`
    Divide,
    /// `^`
    Caret,
    /// `%`
    Percent,

    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `<`
    LessThan,
    /// `<<`
    ShiftLeft,
    /// `>`
    GreaterThan,
    /// `>>`
    ShiftRight,
    /// `>=`
    GreaterThanEqual,
    /// `<=`
    LessThanEqual,

    /// `=`
    Assign,
    /// `=>`
    Arrow,
    /// `->`
    BadArrow,
    /// `+=`
    AddAssign,
    /// `-=`
    SubtractAssign,
    /// `*=`
    MultiplyAssign,
    /// `/=`
    DivideAssign,
    /// `%=`
    RemainderAssign,
    /// `^=`
    ExponentAssign,
    /// `&=`
    BitwiseAndAssign,
    /// `|=`
    BitwiseOrAssign,
    /// `<<=`
    ShiftLeftAssign,
    /// `>>=`
    ShiftRightAssign,
}

/// Literal Token
///
/// NOTE: The suffix is *not* considered when deciding the `LiteralType`.
/// (e.g., float literals like `1f32` are classified by this type as `Int` since they have no `.`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LiteralTokenType {
    /// `12_u8`, `0o100`, `0b120i99`, `1f32`.
    Integer { base: NumberBase, empty_int: bool },
    /// `12.34f32`, `1e3`, but not `1f32`.
    Float {
        base: NumberBase,
        is_empty_exponent: bool,
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
    RawString { hashes: Option<u8> },
    /// `br"abc"`, `br#"abc"#`, `br####"ab"###"c"####`, `br#"a`.
    RawByteString { hashes: Option<u8> },
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
