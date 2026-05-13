use std::fmt::Display;

use serde::{Deserialize, Serialize};

pub use destack_unicode::UNICODE_VERSION;

/// A parsed token.
/// It doesn't contain information about data that has been parsed,
/// only the type of the token and its size.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Token {
    /// The token tag.
    pub ty: TokenType,
    /// The length of the token in bytes.
    pub len: u32,
    /// The literal body of the token.
    pub literal: Option<LiteralType>,
    /// Whether the token is preceded by a line terminator.
    pub is_on_new_line: bool,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<Token {:?}, {}>", self.ty, self.len)
    }
}

impl Token {
    /// Create a token.
    pub const fn new(ty: TokenType, len: u32, literal: Option<LiteralType>) -> Token {
        Token {
            ty,
            len,
            literal,
            is_on_new_line: false,
        }
    }

    /// Create an end token.
    pub const fn end() -> Token {
        Token {
            ty: TokenType::End,
            len: 0,
            literal: None,
            is_on_new_line: false,
        }
    }

    /// Return this token with line boundary information attached.
    #[must_use]
    pub const fn with_on_new_line(mut self, is_on_new_line: bool) -> Token {
        self.is_on_new_line = is_on_new_line;
        self
    }
}

/// Enum representing common lexeme types.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    /// `..=`
    RangeInclusive,
    /// `...`
    Spread,
    /// `->`
    Arrow,
    /// `=>`
    ArrowWide,
    /// `@`
    At,
    /// `#`
    Hash,
    /// `~`
    ElementwiseNot,
    /// `?`
    Maybe,
    /// `??`
    Coalesce,
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
    /// `**`
    Exponent,
    /// `/`
    Divide,
    /// `%`
    Remainder,

    /// --------------------------------------------------
    /// Addition
    /// --------------------------------------------------

    /// `+`
    Add,
    /// `-`
    Subtract,
    /// `++`
    Increment,
    /// `--`
    Decrement,

    /// --------------------------------------------------
    /// Shift
    /// --------------------------------------------------

    /// `<<`
    ShiftLeft,
    /// `>>`
    ShiftRight,
    /// `>>>`
    UnsignedShiftRight,
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
    /// `**=`
    ExponentAssign,
    /// `/=`
    DivideAssign,
    /// `%=`
    RemainderAssign,

    /// --------------------------------------------------
    /// Assignment Addition
    /// --------------------------------------------------

    /// `+=`
    AddAssign,
    /// `-=`
    SubtractAssign,

    /// --------------------------------------------------
    /// Assignment Shift
    /// --------------------------------------------------

    /// `<<=`
    ShiftLeftAssign,
    /// `>>=`
    ShiftRightAssign,
    /// `>>>=`
    UnsignedShiftRightAssign,

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
    /// `??=`
    CoalesceAssign,
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
            TokenType::Colon => write!(f, ":"),
            TokenType::Semicolon => write!(f, ";"),
            TokenType::Comma => write!(f, ","),
            TokenType::Dot => write!(f, "."),
            TokenType::Range => write!(f, ".."),
            TokenType::RangeInclusive => write!(f, "..="),
            TokenType::Spread => write!(f, "..."),
            TokenType::Arrow => write!(f, "->"),
            TokenType::ArrowWide => write!(f, "=>"),
            TokenType::At => write!(f, "@"),
            TokenType::Hash => write!(f, "#"),
            TokenType::ElementwiseNot => write!(f, "~"),
            TokenType::Maybe => write!(f, "?"),
            TokenType::Coalesce => write!(f, "??"),
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
            TokenType::Exponent => write!(f, "**"),
            TokenType::Divide => write!(f, "/"),
            TokenType::Remainder => write!(f, "%"),

            // addition
            TokenType::Add => write!(f, "+"),
            TokenType::Subtract => write!(f, "-"),
            TokenType::Increment => write!(f, "++"),
            TokenType::Decrement => write!(f, "--"),

            // shift
            TokenType::ShiftLeft => write!(f, "<<"),
            TokenType::ShiftRight => write!(f, ">>"),
            TokenType::UnsignedShiftRight => write!(f, ">>>"),

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
            TokenType::ExponentAssign => write!(f, "**="),
            TokenType::DivideAssign => write!(f, "/="),
            TokenType::RemainderAssign => write!(f, "%="),

            // assignment addition
            TokenType::AddAssign => write!(f, "+="),
            TokenType::SubtractAssign => write!(f, "-="),

            // assignment shift
            TokenType::ShiftLeftAssign => write!(f, "<<="),
            TokenType::ShiftRightAssign => write!(f, ">>="),
            TokenType::UnsignedShiftRightAssign => write!(f, ">>>="),

            // assignment elementwise
            TokenType::ElementwiseAndAssign => write!(f, "&="),
            TokenType::ElementwiseXorAssign => write!(f, "^="),
            TokenType::ElementwiseOrAssign => write!(f, "|="),

            // assignment logical
            TokenType::LogicalAndAssign => write!(f, "&&="),
            TokenType::LogicalOrAssign => write!(f, "||="),
            TokenType::CoalesceAssign => write!(f, "??="),
        }
    }
}

/// Literal Token for literal, scalar values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LiteralType {
    /// Boolean (true or false)
    Boolean { value: bool },
    /// Integer (12, 0o100, 0x (is_empty), 0b120, 1.0, 1n)
    Int {
        /// The base of the integer (binary, octal, decimal, hexadecimal).
        base: NumberBase,
        /// Whether the integer is empty (e.g. `0`).
        is_empty: bool,
        /// Whether the integer is a bigint (e.g. `1n`).
        is_bigint: bool,
    },
    /// Float (1.0, 1e3)
    Float {
        /// The base of the float (binary, octal, decimal, hexadecimal).
        base: NumberBase,
        /// Whether the float is empty (e.g. `1.0`).
        is_empty_exponent: bool,
    },
    /// Character ('a', '\\', ''', ';') or HTML entity (`&nbsp;`)
    Character {
        /// Whether the character is terminated.
        is_terminated: bool,
        /// Whether the character is an HTML entity (e.g. `&nbsp;`).
        is_html_entity: bool,
    },
    /// String ("abc", "abc")
    String {
        /// Whether the string is terminated.
        is_terminated: bool,
        /// Whether the string contains invalid escapes like `\8` or `\9`.
        has_invalid_escape: bool,
    },
    /// Regex string (`/abc/`, `/abc/g`, `/abc/i`, `/abc/gi`)
    RegexString {
        /// Whether the regex string has flags (e.g. `/abc/g`).
        has_flags: bool,
    },
    /// Text content inside tree literals (TSX-compatible).
    /// Raw text between `>` and `</` or `{`, like "Hello" in `<div>Hello</div>`.
    /// Separate from String because JSX text has no escape sequences (uses HTML entities instead).
    TreeString,
}

/// Numeric literal base (according to its prefix).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
