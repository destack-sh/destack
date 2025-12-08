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
    /// `*%`
    WrappingMultiply,
    /// `*|`
    SaturatingMultiply,
    /// `**`
    Exponent,
    /// `**%`
    WrappingExponent,
    /// `**|`
    SaturatingExponent,
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
    /// `**=`
    ExponentAssign,
    /// `**%=`
    WrappingExponentAssign,
    /// `**|`
    SaturatingExponentAssign,
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
            TokenType::Wildcard => write!(f, "_"),
            TokenType::Colon => write!(f, ":"),
            TokenType::Semicolon => write!(f, ";"),
            TokenType::Comma => write!(f, ","),
            TokenType::Dot => write!(f, "."),
            TokenType::Range => write!(f, ".."),
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
            TokenType::WrappingMultiply => write!(f, "*%"),
            TokenType::SaturatingMultiply => write!(f, "*|"),
            TokenType::Exponent => write!(f, "**"),
            TokenType::WrappingExponent => write!(f, "**%"),
            TokenType::SaturatingExponent => write!(f, "**|"),
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
            TokenType::ExponentAssign => write!(f, "**="),
            TokenType::WrappingExponentAssign => write!(f, "**%="),
            TokenType::SaturatingExponentAssign => write!(f, "**|=="),
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LiteralType {
    /// Boolean (true or false)
    Boolean { value: bool },
    /// Integer (12, 0o100, 0x (is_empty), 0b120, 1.0, 1n)
    Int {
        base: NumberBase,
        is_empty: bool,
        is_bigint: bool,
    },
    /// Float (1.0, 1e3)
    Float {
        base: NumberBase,
        is_empty_exponent: bool,
    },
    /// Character ('a', '\\', ''', ';') or HTML entity (`&nbsp;`)
    Character {
        is_terminated: bool,
        is_html_entity: bool,
    },
    /// String ("abc", "abc")
    String { is_terminated: bool },
    /// Regex string (`/abc/`, `/abc/g`, `/abc/i`, `/abc/gi`)
    RegexString { has_flags: bool },
    /// Text content inside tree literals (TSX-compatible).
    /// Raw text between `>` and `</` or `{`, like "Hello" in `<div>Hello</div>`.
    TreeString, // NOTE #Cleanup: does TreeString need to be a separate literal type?
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
