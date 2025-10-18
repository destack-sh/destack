use std::fmt::Display;

pub use destack_unicode::UNICODE_VERSION;

/// A parsed Token.
/// It doesn't contain information about data that has been parsed,
/// only the type of the token and its size.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Token {
    /// The Token tag.
    pub ty: TokenType,
    /// The length of the token in bytes.
    pub len: u32,
    /// The literal body of the token.
    pub literal: Option<LiteralType> = None,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<Token {:?}, {}>", self.ty, self.len)
    }
}

impl Token {
    pub const fn new(ty: TokenType, len: u32, literal: Option<LiteralType>) -> Token {
        Token { ty, len, literal }
    }

    pub const fn end() -> Token {
        Token {
            ty: TokenType::End,
            len: 0,
            literal: None,
        }
    }
}

/// Enum representing common lexeme types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenType {
    /// --------------------------------------------------
    /// Structural
    /// --------------------------------------------------

    /// Newline character.
    Newline,
    /// Non-newline whitespace character sequence.
    Whitespace,
    /// Unknown/Unexpected (e.g., '№')
    Unknown,
    /// End of sequence (e.g., end of source file)
    End,

    /// --------------------------------------------------
    /// Annotations
    /// --------------------------------------------------

    /// Line comment, e.g. `// comment` `//// comment` `//////// comment`.
    LineComment,
    /// Block comment, e.g. `/* comment */`
    BlockComment,
    /// Doc line comment with exactly three slashes, e.g. `/// doc comment` or just `///`
    DocLineComment,
    /// Doc block comment, e.g. `/** doc comment *//`
    DocBlockComment,
    // (tags are not parsed as tokens)

    /// --------------------------------------------------
    /// Identifiers / Literals
    /// --------------------------------------------------

    /// Identifier or keyword, e.g. `identifier` or `continue`.
    Identifier,
    /// Identifier that is invalid (e.g. because it contains emoji).
    InvalidIdentifier,
    /// Unknown literal prefix, like `foo#`, `foo'`, `foo"`.
    UnknownLiteralPrefix,
    /// "Raw" Literals, e.g. `12`, `1.0e-40`, `b"123"`.
    Literal,
    /// Template string start (`start${` in `${start}middle${end}`)
    TemplateStringStart,
    /// Template string middle (`}middle${` in `${start}middle${end}`)
    TemplateStringMiddle,
    /// Template string end (`}end` in `${start}middle${end}`)
    TemplateStringEnd,
    /// Template string without interpolation (`no interpolation`)
    TemplateString,

    /// --------------------------------------------------
    /// Symbols
    /// --------------------------------------------------

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
    /// `->`
    Arrow,
    /// `=>`
    ArrowWide,
    /// `@`
    At,
    /// `#`
    Tag,
    /// `~`
    ElementwiseNot,
    /// `?`
    Maybe,
    /// `??`
    Coalesce,
    /// `$`
    Dynamic,
    /// `!`
    Not,

    /// --------------------------------------------------
    /// Parentheses
    /// --------------------------------------------------

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

    /// --------------------------------------------------
    /// Multiplication
    /// --------------------------------------------------

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

    /// --------------------------------------------------
    /// Addition
    /// --------------------------------------------------

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
    /// `++`
    Increment,
    /// `--`
    Decrement,

    /// --------------------------------------------------
    /// Shift
    /// --------------------------------------------------

    /// `<<`
    ShiftLeft,
    /// `<<|`
    SaturatingShiftLeft,
    // NOTE: we don't have a `>>` token to avoid ambiguity in static parameters
    //  (otherwise we would have to perform some ugly "ungluing" which is cumbersome)

    /// --------------------------------------------------
    /// Elementwise
    /// --------------------------------------------------

    /// `&`
    ElementwiseAnd,
    /// `^`
    ElementwiseXor,
    /// `|`
    ElementwiseOr,

    /// --------------------------------------------------
    /// Comparison
    /// --------------------------------------------------

    /// `==`
    Equal,
    /// `===`
    EqualWide,
    /// `!=`
    NotEqual,
    /// `!==`
    NotEqualWide,
    /// `<`
    LessThan,
    /// `<=`
    LessThanOrEqual,
    /// `>`
    GreaterThan,
    /// `>=`
    GreaterThanOrEqual,

    /// --------------------------------------------------
    /// Logical
    /// --------------------------------------------------

    /// `&&`
    LogicalAnd,
    /// `||`
    LogicalOr,

    /// --------------------------------------------------
    /// Assignment
    /// --------------------------------------------------

    /// `=`
    Assign,

    /// --------------------------------------------------
    /// Assignment Multiplication
    /// --------------------------------------------------

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

    /// --------------------------------------------------
    /// Assignment Addition
    /// --------------------------------------------------

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

    /// --------------------------------------------------
    /// Assignment Shift
    /// --------------------------------------------------

    /// `<<=`
    ShiftLeftAssign,
    /// `<<|=`
    SaturatingShiftLeftAssign,
    /// `>>=`
    ShiftRightAssign,

    /// --------------------------------------------------
    /// Assignment Elementwise
    /// --------------------------------------------------

    /// `&=`
    ElementwiseAndAssign,
    /// `^=`
    ElementwiseXorAssign,
    /// `|=`
    ElementwiseOrAssign,

    /// --------------------------------------------------
    /// Assignment Logical
    /// --------------------------------------------------

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
            TokenType::TemplateStringStart => write!(f, "TemplateStringStart"),
            TokenType::TemplateStringMiddle => write!(f, "TemplateStringMiddle"),
            TokenType::TemplateStringEnd => write!(f, "TemplateStringEnd"),
            TokenType::TemplateString => write!(f, "TemplateString"),

            // symbols
            TokenType::Wildcard => write!(f, "_"),
            TokenType::Colon => write!(f, ":"),
            TokenType::Semicolon => write!(f, ";"),
            TokenType::Comma => write!(f, ","),
            TokenType::Dot => write!(f, "."),
            TokenType::Range => write!(f, ".."),
            TokenType::RangeWide => write!(f, "..."),
            TokenType::Arrow => write!(f, "->"),
            TokenType::ArrowWide => write!(f, "=>"),
            TokenType::At => write!(f, "@"),
            TokenType::Tag => write!(f, "#"),
            TokenType::ElementwiseNot => write!(f, "~"),
            TokenType::Maybe => write!(f, "?"),
            TokenType::Coalesce => write!(f, "??"),
            TokenType::Dynamic => write!(f, "$"),
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
            TokenType::Increment => write!(f, "++"),
            TokenType::Decrement => write!(f, "--"),

            // shift
            TokenType::ShiftLeft => write!(f, "<<"),
            TokenType::SaturatingShiftLeft => write!(f, "<<|"),

            // elementwise
            TokenType::ElementwiseAnd => write!(f, "&"),
            TokenType::ElementwiseXor => write!(f, "^"),
            TokenType::ElementwiseOr => write!(f, "|"),

            // comparison
            TokenType::Equal => write!(f, "=="),
            TokenType::EqualWide => write!(f, "==="),
            TokenType::NotEqual => write!(f, "!="),
            TokenType::NotEqualWide => write!(f, "!=="),
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

            // assignment elementwise
            TokenType::ElementwiseAndAssign => write!(f, "&="),
            TokenType::ElementwiseXorAssign => write!(f, "^="),
            TokenType::ElementwiseOrAssign => write!(f, "|="),

            // assignment logical
            TokenType::LogicalAndAssign => write!(f, "&&="),
            TokenType::LogicalOrAssign => write!(f, "||="),
        }
    }
}

/// Literal Token for literal, scalar values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LiteralType {
    /// Boolean (true or false)
    Boolean { value: bool },
    /// Integer (12, 0o100, 0x (is_empty), 0b120, 1.0)
    Int { base: NumberBase, is_empty: bool },
    /// Float (1.0, 1e3)
    Float {
        base: NumberBase,
        is_empty_exponent: bool,
    },
    /// Character ('a', '\\', ''', ';')
    Character { is_terminated: bool },
    /// Byte string (b'a', b'\\', b''', b';)
    Byte { is_terminated: bool },
    /// String ("abc", "abc")
    String { is_terminated: bool },
    /// Raw string (r"abc", r#"abc"#, r####"ab"###"c"####, r#"a")
    RawString { hashes: Option<u8> },
    /// Regex string (`/abc/`, `/abc/g`, `/abc/i`, `/abc/gi`)
    RegexString { has_flags: bool },
    /// Byte string (b"abc", b"abc")
    ByteString { is_terminated: bool },
    /// Raw byte string (br"abc", br#"abc"#, br####"ab"###"c"####, br#"a")
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
