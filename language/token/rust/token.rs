use std::fmt::Display;

pub use destack_library_unicode::UNICODE_VERSION;

/// A parsed Token.
/// It doesn't contain information about data that has been parsed,
/// only the type of the token and its size.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenType {
    /// A newline character.
    Newline,
    /// Any non-newline whitespace character sequence.
    Whitespace,
    /// Unknown/Unexpected (e.g., '№')
    Unknown,
    /// End of sequence (e.g., end of source file)
    End,

    // comments
    /// A line comment, e.g. `// comment` `//// comment` `//////// comment`.
    LineComment,
    /// A block comment, e.g. `/* comment */`
    BlockComment,
    /// A doc line comment with exactly three slashes, e.g. `/// doc comment` or just `///`
    DocLineComment,
    /// A doc block comment, e.g. `/** doc comment *//`
    DocBlockComment,

    // identifiers / literals
    /// An identifier or keyword, e.g. `identifier` or `continue`.
    Identifier,
    /// An identifier that is invalid (e.g. because it contains emoji).
    InvalidIdentifier,
    /// An unknown literal prefix, like `foo#`, `foo'`, `foo"`.
    UnknownLiteralPrefix,
    /// "Raw" Literals, e.g. `12`, `1.0e-40`, `b"123"`.
    Literal,

    // symbols
    /// `:`
    Colon,
    /// `::`
    DoubleColon,
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
    /// `#`
    Pound,
    /// '---',
    Empty,
    /// `=>`
    Arrow,
    /// `->`
    BadArrow,
    /// `@`
    At,
    /// `~`
    BitwiseNot,
    /// `?`
    Question,
    /// `$`
    Dollar,
    /// `!`
    Bang,

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

/// "Raw" Literal Token for literal, scalar values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RawLiteralType {
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
