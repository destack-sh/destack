use std::fmt::Display;

pub use destack_unicode::UNICODE_VERSION;

/// A parsed Token.
/// It doesn't contain information about data that has been parsed,
/// only the type of the token and its size.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Token {
    /// The Token tag.
    pub r#type: TokenType,
    /// The length of the token in bytes.
    pub len: u32,
    /// The literal body of the token.
    pub body: Option<RawLiteralType> = None,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<Token {:?}, {}>", self.r#type, self.len)
    }
}

impl Token {
    pub const fn new(r#type: TokenType, len: u32, body: Option<RawLiteralType>) -> Token {
        Token { r#type, len, body }
    }

    pub const fn eof() -> Token {
        Token {
            r#type: TokenType::End,
            len: 0,
            body: None,
        }
    }
}

/// Enum representing common lexeme types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenType {
    /// Newline character.
    Newline,
    /// Non-newline whitespace character sequence.
    Whitespace,
    /// Unknown/Unexpected (e.g., '№')
    Unknown,
    /// End of sequence (e.g., end of source file)
    End,

    // annotations
    /// Line comment, e.g. `// comment` `//// comment` `//////// comment`.
    LineComment,
    /// Block comment, e.g. `/* comment */`
    BlockComment,
    /// Doc line comment with exactly three slashes, e.g. `/// doc comment` or just `///`
    DocLineComment,
    /// Doc block comment, e.g. `/** doc comment *//`
    DocBlockComment,
    // (tags are not parsed as tokens)

    // identifiers / literals
    /// Identifier or keyword, e.g. `identifier` or `continue`.
    Identifier,
    /// Identifier that is invalid (e.g. because it contains emoji).
    InvalidIdentifier,
    /// Unknown literal prefix, like `foo#`, `foo'`, `foo"`.
    UnknownLiteralPrefix,
    /// "Raw" Literals, e.g. `12`, `1.0e-40`, `b"123"`.
    Literal,

    // symbols
    /// Wildcard literal `_`.
    Wildcard,
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
    RangeWide,
    /// `=>`
    FatArrow,
    /// `->`
    ThinArrow,
    /// `@`
    At,
    /// `#`
    Tag,
    /// `~`
    BitwiseNot,
    /// `?`
    Maybe,
    /// `??`
    Coalesce,
    /// `$`
    Virtual,
    /// `!`
    Not,

    // parentheses
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

    // multiplication
    /// `*`
    Multiply,
    /// `*%`
    WrappingMultiply,
    /// `*|`
    SaturatingMultiply,
    /// `/`
    Divide,
    /// `%`
    Remainder,

    // addition
    /// `+`
    Add,
    /// `+%`
    WrappingAdd,
    /// `+|`
    SaturatingAdd,
    /// `-`
    Subtract,
    /// `-%`
    WrappingSubtract,
    /// `-|`
    SaturatingSubtract,

    // shift
    /// `<<`
    ShiftLeft,
    /// `<<|`
    SaturatingShiftLeft,
    /// `>>`
    ShiftRight,

    // bitwise
    /// `&`
    BitwiseAnd,
    /// `^`
    BitwiseXor,
    /// `|`
    BitwiseOr,

    // comparison
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `<`
    LessThan,
    /// `<=`
    LessThanOrEqual,
    /// `>`
    GreaterThan,
    /// `>=`
    GreaterThanOrEqual,

    // logical
    /// `&&`
    LogicalAnd,
    /// `||`
    LogicalOr,

    // assignment
    /// `=`
    Assign,

    // assignment multiplication
    /// `*=`
    MultiplyAssign,
    /// `*%=`
    WrappingMultiplyAssign,
    /// `*|=`
    SaturatingMultiplyAssign,
    /// `/=`
    DivideAssign,
    /// `%=`
    RemainderAssign,

    // assignment addition
    /// `+=`
    AddAssign,
    /// `+%=`
    WrappingAddAssign,
    /// `+|=`
    SaturatingAddAssign,
    /// `-=`
    SubtractAssign,
    /// `-%=`
    WrappingSubtractAssign,
    /// `-|=`
    SaturatingSubtractAssign,

    // assignment shift
    /// `<<=`
    ShiftLeftAssign,
    /// `<<|=`
    SaturatingShiftLeftAssign,
    /// `>>=`
    ShiftRightAssign,

    // assignment bitwise
    /// `&=`
    BitwiseAndAssign,
    /// `^=`
    BitwiseXorAssign,
    /// `|=`
    BitwiseOrAssign,

    // assignment logical
    /// `&&=`
    LogicalAndAssign,
    /// `||=`
    LogicalOrAssign,
}

impl Display for TokenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // structural
            TokenType::Newline => write!(f, "Newline"),
            TokenType::Whitespace => write!(f, "Whitespace"),
            TokenType::Unknown => write!(f, "Unknown"),
            TokenType::End => write!(f, "End"),

            // annotations
            TokenType::LineComment => write!(f, "//"),
            TokenType::BlockComment => write!(f, "/*"),
            TokenType::DocLineComment => write!(f, "///"),
            TokenType::DocBlockComment => write!(f, "/**"),

            // identifiers / literals
            TokenType::Identifier => write!(f, "Identifier"),
            TokenType::InvalidIdentifier => write!(f, "InvalidIdentifier"),
            TokenType::UnknownLiteralPrefix => write!(f, "UnknownLiteralPrefix"),
            TokenType::Literal => write!(f, "Literal"),

            // symbols
            TokenType::Wildcard => write!(f, "_"),
            TokenType::Colon => write!(f, ":"),
            TokenType::Semicolon => write!(f, ";"),
            TokenType::Comma => write!(f, ","),
            TokenType::Dot => write!(f, "."),
            TokenType::Range => write!(f, ".."),
            TokenType::RangeWide => write!(f, "..."),
            TokenType::FatArrow => write!(f, "=>"),
            TokenType::ThinArrow => write!(f, "->"),
            TokenType::At => write!(f, "@"),
            TokenType::Tag => write!(f, "#"),
            TokenType::BitwiseNot => write!(f, "~"),
            TokenType::Maybe => write!(f, "?"),
            TokenType::Coalesce => write!(f, "??"),
            TokenType::Virtual => write!(f, "$"),
            TokenType::Not => write!(f, "!"),

            // parentheses
            TokenType::OpenParenthesis => write!(f, "("),
            TokenType::CloseParenthesis => write!(f, ")"),
            TokenType::OpenBrace => write!(f, "{{"),
            TokenType::CloseBrace => write!(f, "}}"),
            TokenType::OpenBracket => write!(f, "["),
            TokenType::CloseBracket => write!(f, "]"),

            // multiplication
            TokenType::Multiply => write!(f, "*"),
            TokenType::WrappingMultiply => write!(f, "*%"),
            TokenType::SaturatingMultiply => write!(f, "*|"),
            TokenType::Divide => write!(f, "/"),
            TokenType::Remainder => write!(f, "%"),

            // addition
            TokenType::Add => write!(f, "+"),
            TokenType::WrappingAdd => write!(f, "+%"),
            TokenType::SaturatingAdd => write!(f, "+|"),
            TokenType::Subtract => write!(f, "-"),
            TokenType::WrappingSubtract => write!(f, "-%"),
            TokenType::SaturatingSubtract => write!(f, "-|"),

            // shift
            TokenType::ShiftLeft => write!(f, "<<"),
            TokenType::SaturatingShiftLeft => write!(f, "<<|"),
            TokenType::ShiftRight => write!(f, ">>"),

            // bitwise
            TokenType::BitwiseAnd => write!(f, "&"),
            TokenType::BitwiseXor => write!(f, "^"),
            TokenType::BitwiseOr => write!(f, "|"),

            // comparison
            TokenType::Equal => write!(f, "=="),
            TokenType::NotEqual => write!(f, "!="),
            TokenType::LessThan => write!(f, "<"),
            TokenType::LessThanOrEqual => write!(f, "<="),
            TokenType::GreaterThan => write!(f, ">"),
            TokenType::GreaterThanOrEqual => write!(f, ">="),

            // logical
            TokenType::LogicalAnd => write!(f, "&&"),
            TokenType::LogicalOr => write!(f, "||"),

            // assignment
            TokenType::Assign => write!(f, "="),

            // assignment multiplication
            TokenType::MultiplyAssign => write!(f, "*="),
            TokenType::WrappingMultiplyAssign => write!(f, "*%="),
            TokenType::SaturatingMultiplyAssign => write!(f, "*|=="),
            TokenType::DivideAssign => write!(f, "/="),
            TokenType::RemainderAssign => write!(f, "%="),

            // assignment addition
            TokenType::AddAssign => write!(f, "+="),
            TokenType::WrappingAddAssign => write!(f, "+%="),
            TokenType::SaturatingAddAssign => write!(f, "+|=="),
            TokenType::SubtractAssign => write!(f, "-="),
            TokenType::WrappingSubtractAssign => write!(f, "-%="),
            TokenType::SaturatingSubtractAssign => write!(f, "-|=="),

            // assignment shift
            TokenType::ShiftLeftAssign => write!(f, "<<="),
            TokenType::SaturatingShiftLeftAssign => write!(f, "<<|=="),
            TokenType::ShiftRightAssign => write!(f, ">>="),

            // assignment bitwise
            TokenType::BitwiseAndAssign => write!(f, "&="),
            TokenType::BitwiseXorAssign => write!(f, "^="),
            TokenType::BitwiseOrAssign => write!(f, "|="),

            // assignment logical
            TokenType::LogicalAndAssign => write!(f, "&&="),
            TokenType::LogicalOrAssign => write!(f, "||="),
        }
    }
}

/// "Raw" Literal Token for literal, scalar values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RawLiteralType {
    /// Void
    Void,
    /// Null
    Null,
    /// Boolean
    Boolean { value: bool },
    /// 12, 0o100, 0x (is_empty), 0b120, 1.0
    Int { base: NumberBase, is_empty: bool },
    /// 1.0, 1e3
    Float {
        base: NumberBase,
        is_empty_exponent: bool,
    },
    /// 'a', '\\', ''', ';
    Character { is_terminated: bool },
    /// b'a', b'\\', b''', b';
    Byte { is_terminated: bool },
    /// "abc", "abc
    String { is_terminated: bool },
    /// b"abc", b"abc
    ByteString { is_terminated: bool },
    /// r"abc", r#"abc"#, r####"ab"###"c"####, r#"a
    RawString { hashes: Option<u8> },
    /// br"abc", br#"abc"#, br####"ab"###"c"####, br#"a
    RawByteString { hashes: Option<u8> },
}

/// An error from parsing a raw string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RawStringError {
    /// Non `#` characters exist between `r` and `"`, e.g. `r##~"abcde"##`
    InvalidStarter { bad_char: char },
    /// The string was not terminated, e.g. `r###"abcde"##`.
    /// `possible_terminator_offset` is the number of characters after `r` or
    /// `br` where they may have intended to terminate it.
    NoTerminator {
        expected_hashes: u32,
        found_hashes: u32,
        possible_terminator_offset: Option<u32>,
    },
    /// More than max_hashes `#`s exist.
    TooManyDelimiters { found_hashes: u32 },
}

/// Numeric literal base (according to its prefix).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
