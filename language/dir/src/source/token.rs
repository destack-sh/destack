use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use tspp_serde::Reflect;

use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tspp_source::{ByteRange, FileId, Span};

use super::Keyword;

pub use tspp_unicode::UNICODE_VERSION;

const TOKEN_TYPE_BITS: u32 = 0x0000_00ff;
const TOKEN_LINE_BIT: u32 = 0x0000_0100;
const TOKEN_KEYWORD_BITS: u32 = 0x0000_fe00;
const TOKEN_KEYWORD_SHIFT: u32 = 9;
const TOKEN_IDENTIFIER_ESCAPE_BIT: u32 = 0x0001_0000;
const TOKEN_LITERAL_SHIFT: u32 = 16;
const TOKEN_NON_KEYWORD_CODE: u8 = 0x7f;
const TOKEN_TYPE_MAX: u8 = TokenType::CoalesceAssign as u8;
const LITERAL_KIND_BITS: u16 = 0x000f;
const LITERAL_FLAG_A: u16 = 0x0010;
const LITERAL_FLAG_B: u16 = 0x0020;
const LITERAL_BASE_SHIFT: u16 = 6;

const _: () = assert!(Keyword::With as u8 + 1 < TOKEN_NON_KEYWORD_CODE);

/// A source Token.
#[derive(Copy, Clone)]
pub struct Token {
    /// The start byte of the token in its source file.
    start: u32,
    /// The length of the token in bytes.
    len: u32,
    /// Packed token type, line boundary flag, keyword cache, and literal metadata.
    bits: u32,
}

impl PartialEq for Token {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start
            && self.len == other.len
            && self.identity_bits() == other.identity_bits()
    }
}

impl Eq for Token {}

impl Hash for Token {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.start.hash(state);
        self.len.hash(state);
        self.identity_bits().hash(state);
    }
}

/// Serializable token record.
#[derive(Deserialize, Reflect)]
struct TokenRecord {
    /// The start byte of the token in its source file.
    start: u32,
    /// The token tag.
    ty: TokenType,
    /// The length of the token in bytes.
    len: u32,
    /// The literal body of the token.
    literal: Option<TokenLiteral>,
    /// Whether the token is preceded by a line terminator.
    is_on_new_line: bool,
}

impl Reflect for Token {
    fn reflect(schema: &mut tspp_serde::Schema) -> tspp_serde::Type {
        TokenRecord::reflect(schema)
    }
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Token")
            .field("start", &self.start())
            .field("ty", &self.ty())
            .field("len", &self.len())
            .field("keyword", &self.keyword())
            .field("literal", &self.literal())
            .field("is_on_new_line", &self.is_on_new_line())
            .finish()
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<Token {:?}, {}..{}>",
            self.ty(),
            self.start(),
            self.end()
        )
    }
}

impl Serialize for Token {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut token = serializer.serialize_struct("Token", 5)?;
        token.serialize_field("start", &self.start())?;
        token.serialize_field("ty", &self.ty())?;
        token.serialize_field("len", &self.len())?;
        token.serialize_field("literal", &self.literal())?;
        token.serialize_field("is_on_new_line", &self.is_on_new_line())?;
        token.end()
    }
}

impl<'de> Deserialize<'de> for Token {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let record = TokenRecord::deserialize(deserializer)?;

        Ok(
            Token::new(record.ty, record.start, record.len, record.literal)
                .with_on_new_line(record.is_on_new_line),
        )
    }
}

impl Token {
    /// Return token bits without identifier-only lexer caches.
    const fn identity_bits(self) -> u32 {
        if self.is(TokenType::Identifier) {
            self.bits & !(TOKEN_KEYWORD_BITS | TOKEN_IDENTIFIER_ESCAPE_BIT)
        } else {
            self.bits
        }
    }

    /// Create a token.
    #[inline(always)]
    pub const fn new(ty: TokenType, start: u32, len: u32, literal: Option<TokenLiteral>) -> Token {
        let literal = match literal {
            Some(literal) => literal.bits(),
            None => 0,
        } as u32;
        let token_type = ty as u32;
        let bits = token_type | (literal << TOKEN_LITERAL_SHIFT);

        Token { start, len, bits }
    }

    /// Create a token without literal metadata.
    #[inline(always)]
    pub const fn simple(ty: TokenType, start: u32, len: u32) -> Token {
        Token {
            start,
            len,
            bits: ty as u32,
        }
    }

    /// Create an identifier token with optional keyword identity.
    #[inline(always)]
    pub const fn identifier(
        start: u32,
        len: u32,
        keyword: Option<Keyword>,
        is_escaped: bool,
    ) -> Token {
        let mut token = Token::simple(TokenType::Identifier, start, len);
        let code = match keyword {
            Some(keyword) => keyword.code() + 1,
            None => TOKEN_NON_KEYWORD_CODE,
        } as u32;
        token.bits |= code << TOKEN_KEYWORD_SHIFT;
        if is_escaped {
            token.bits |= TOKEN_IDENTIFIER_ESCAPE_BIT;
        }

        token
    }

    /// Return whether this identifier contains a Unicode escape.
    #[inline]
    pub const fn is_identifier_escaped(self) -> bool {
        self.is(TokenType::Identifier) && self.bits & TOKEN_IDENTIFIER_ESCAPE_BIT != 0
    }

    /// Create an end token.
    #[inline(always)]
    pub const fn eof(start: u32) -> Token {
        Token::simple(TokenType::End, start, 0)
    }

    /// Return the token tag.
    #[inline(always)]
    pub const fn ty(self) -> TokenType {
        let code = (self.bits & TOKEN_TYPE_BITS) as u8;
        debug_assert!(
            code <= TOKEN_TYPE_MAX,
            "packed token type code must be valid"
        );

        // token bits are private and only built from TokenType values
        unsafe { std::mem::transmute::<u8, TokenType>(code) }
    }

    /// Return whether this token has the given tag.
    #[inline]
    pub const fn is(self, token_type: TokenType) -> bool {
        self.bits & TOKEN_TYPE_BITS == token_type as u32
    }

    /// Return whether this token is semantic source content.
    #[inline]
    pub const fn is_semantic(self) -> bool {
        self.ty().is_semantic()
    }

    /// Return the start byte in the source file.
    #[inline]
    pub const fn start(self) -> u32 {
        self.start
    }

    /// Return the exclusive end byte in the source file.
    #[inline]
    pub const fn end(self) -> u32 {
        self.start + self.len
    }

    /// Return the token byte range in its source file.
    #[inline]
    pub const fn range(self) -> ByteRange {
        ByteRange {
            start: self.start,
            end: self.end(),
        }
    }

    /// Return the source span for this token in one file.
    #[inline]
    pub fn span(self, file: FileId) -> Span {
        Span::new(file, self.start, self.end())
    }

    /// Return the length of the token in bytes.
    #[inline]
    pub const fn len(self) -> u32 {
        self.len
    }

    /// Return whether the token is empty.
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// Return the literal body of the token.
    #[inline]
    pub const fn literal(self) -> Option<TokenLiteral> {
        let code = (self.bits >> TOKEN_LITERAL_SHIFT) as u16;

        TokenLiteral::from_bits(code)
    }

    /// Return the keyword identity carried by this identifier token.
    #[inline]
    pub fn keyword(self) -> Option<Keyword> {
        if !self.is(TokenType::Identifier) {
            return None;
        }

        let code = ((self.bits & TOKEN_KEYWORD_BITS) >> TOKEN_KEYWORD_SHIFT) as u8;
        if code == 0 {
            return None;
        }

        Keyword::from_code(code - 1)
    }

    /// Return the keyword classification carried by this identifier token.
    #[inline]
    pub fn classified_keyword(self) -> Option<Option<Keyword>> {
        if !self.is(TokenType::Identifier) {
            return None;
        }

        let code = ((self.bits & TOKEN_KEYWORD_BITS) >> TOKEN_KEYWORD_SHIFT) as u8;
        if code == 0 {
            return None;
        }

        if code == TOKEN_NON_KEYWORD_CODE {
            return Some(None);
        }

        Some(Keyword::from_code(code - 1))
    }

    /// Return whether the token is preceded by a line terminator.
    #[inline]
    pub const fn is_on_new_line(self) -> bool {
        self.bits & TOKEN_LINE_BIT != 0
    }

    /// Return this token with line boundary information attached.
    #[must_use]
    pub const fn with_on_new_line(mut self, is_on_new_line: bool) -> Token {
        if is_on_new_line {
            self.bits |= TOKEN_LINE_BIT;
        } else {
            self.bits &= !TOKEN_LINE_BIT;
        }
        self
    }
}

/// An invalid packed token type code.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TokenTypeCodeError;

/// Enum representing common lexeme types.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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
    /// Lifetime name, e.g. `'a` or `'static`.
    Lifetime,

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

impl TryFrom<u8> for TokenType {
    type Error = TokenTypeCodeError;

    /// Convert a packed token type code into a token type.
    fn try_from(code: u8) -> Result<Self, Self::Error> {
        match TokenType::from_code(code) {
            Some(token_type) => Ok(token_type),
            None => Err(TokenTypeCodeError),
        }
    }
}

impl TokenType {
    /// Return whether this token contains an authored literal value.
    #[inline]
    pub const fn is_literal(self) -> bool {
        matches!(
            self,
            Self::Literal
                | Self::TemplateStringStart
                | Self::TemplateStringMiddle
                | Self::TemplateStringEnd
                | Self::TemplateString
        )
    }

    /// Return whether this token type is semantic source content.
    #[inline]
    pub const fn is_semantic(self) -> bool {
        !matches!(
            self,
            Self::Newline
                | Self::Whitespace
                | Self::LineComment
                | Self::BlockComment
                | Self::DocLineComment
                | Self::DocBlockComment
        )
    }

    /// Convert a dense token type code into a token type.
    #[inline(always)]
    pub fn from_code(code: u8) -> Option<Self> {
        if code > TOKEN_TYPE_MAX {
            return None;
        }

        // token types are a dense repr(u8) enum from 0 through TOKEN_TYPE_MAX
        Some(unsafe { std::mem::transmute::<u8, TokenType>(code) })
    }
}

impl From<TokenType> for u8 {
    /// Convert a token type into its packed token type code.
    fn from(token_type: TokenType) -> Self {
        token_type as u8
    }
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
            TokenType::Lifetime => write!(f, "Lifetime"),

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
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
pub enum TokenLiteral {
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
    /// Text content inside tree literals.
    /// Raw text between `>` and `</` or `{`, like "Hello" in `<div>Hello</div>`.
    /// Separate from String because tree text uses HTML entities instead of escape sequences.
    TreeString,
}

impl TokenLiteral {
    /// Return this token literal as compact token metadata.
    const fn bits(self) -> u16 {
        match self {
            TokenLiteral::Boolean { value } => 1 | flag_bit(value, LITERAL_FLAG_A),
            TokenLiteral::Int {
                base,
                is_empty,
                is_bigint,
            } => {
                2 | (base.bits() << LITERAL_BASE_SHIFT)
                    | flag_bit(is_empty, LITERAL_FLAG_A)
                    | flag_bit(is_bigint, LITERAL_FLAG_B)
            }
            TokenLiteral::Float {
                base,
                is_empty_exponent,
            } => {
                3 | (base.bits() << LITERAL_BASE_SHIFT)
                    | flag_bit(is_empty_exponent, LITERAL_FLAG_A)
            }
            TokenLiteral::Character {
                is_terminated,
                is_html_entity,
            } => {
                4 | flag_bit(is_terminated, LITERAL_FLAG_A)
                    | flag_bit(is_html_entity, LITERAL_FLAG_B)
            }
            TokenLiteral::String {
                is_terminated,
                has_invalid_escape,
            } => {
                5 | flag_bit(is_terminated, LITERAL_FLAG_A)
                    | flag_bit(has_invalid_escape, LITERAL_FLAG_B)
            }
            TokenLiteral::RegexString { has_flags } => 6 | flag_bit(has_flags, LITERAL_FLAG_A),
            TokenLiteral::TreeString => 7,
        }
    }

    /// Return the token literal represented by compact token metadata.
    const fn from_bits(bits: u16) -> Option<Self> {
        match bits & LITERAL_KIND_BITS {
            0 => None,
            1 => Some(TokenLiteral::Boolean {
                value: bits & LITERAL_FLAG_A != 0,
            }),
            2 => Some(TokenLiteral::Int {
                base: NumberBase::from_bits((bits >> LITERAL_BASE_SHIFT) & 0x0003),
                is_empty: bits & LITERAL_FLAG_A != 0,
                is_bigint: bits & LITERAL_FLAG_B != 0,
            }),
            3 => Some(TokenLiteral::Float {
                base: NumberBase::from_bits((bits >> LITERAL_BASE_SHIFT) & 0x0003),
                is_empty_exponent: bits & LITERAL_FLAG_A != 0,
            }),
            4 => Some(TokenLiteral::Character {
                is_terminated: bits & LITERAL_FLAG_A != 0,
                is_html_entity: bits & LITERAL_FLAG_B != 0,
            }),
            5 => Some(TokenLiteral::String {
                is_terminated: bits & LITERAL_FLAG_A != 0,
                has_invalid_escape: bits & LITERAL_FLAG_B != 0,
            }),
            6 => Some(TokenLiteral::RegexString {
                has_flags: bits & LITERAL_FLAG_A != 0,
            }),
            7 => Some(TokenLiteral::TreeString),
            _ => None,
        }
    }
}

/// Numeric literal base (according to its prefix).
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Reflect,
)]
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

impl NumberBase {
    /// Return this number base as compact token metadata.
    const fn bits(self) -> u16 {
        match self {
            NumberBase::Binary => 0,
            NumberBase::Octal => 1,
            NumberBase::Decimal => 2,
            NumberBase::Hexadecimal => 3,
        }
    }

    /// Return the number base represented by compact token metadata.
    const fn from_bits(bits: u16) -> NumberBase {
        match bits {
            0 => NumberBase::Binary,
            1 => NumberBase::Octal,
            2 => NumberBase::Decimal,
            _ => NumberBase::Hexadecimal,
        }
    }
}

/// Return a bit when a compact token flag is set.
const fn flag_bit(is_set: bool, bit: u16) -> u16 {
    if is_set { bit } else { 0 }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::mem::size_of;

    use super::*;

    #[test]
    fn test_pack_token_in_twelve_bytes() {
        assert_eq!(size_of::<Token>(), 12);
    }

    #[test]
    fn test_roundtrip_token_literals() {
        let literals = [
            None,
            Some(TokenLiteral::Boolean { value: false }),
            Some(TokenLiteral::Boolean { value: true }),
            Some(TokenLiteral::Int {
                base: NumberBase::Binary,
                is_empty: false,
                is_bigint: false,
            }),
            Some(TokenLiteral::Int {
                base: NumberBase::Octal,
                is_empty: true,
                is_bigint: false,
            }),
            Some(TokenLiteral::Int {
                base: NumberBase::Decimal,
                is_empty: false,
                is_bigint: true,
            }),
            Some(TokenLiteral::Int {
                base: NumberBase::Hexadecimal,
                is_empty: true,
                is_bigint: true,
            }),
            Some(TokenLiteral::Float {
                base: NumberBase::Decimal,
                is_empty_exponent: true,
            }),
            Some(TokenLiteral::Character {
                is_terminated: true,
                is_html_entity: false,
            }),
            Some(TokenLiteral::String {
                is_terminated: true,
                has_invalid_escape: true,
            }),
            Some(TokenLiteral::RegexString { has_flags: true }),
            Some(TokenLiteral::TreeString),
        ];

        for literal in literals {
            let token = Token::new(TokenType::Literal, 0, 7, literal).with_on_new_line(true);

            assert_eq!(token.ty(), TokenType::Literal);
            assert_eq!(token.len(), 7);
            assert_eq!(token.literal(), literal);
            assert!(token.is_on_new_line());
        }
    }

    #[test]
    fn test_distinguish_literal_identity_from_identifier_caches() {
        let character = Token::new(
            TokenType::Literal,
            0,
            1,
            Some(TokenLiteral::Character {
                is_terminated: false,
                is_html_entity: false,
            }),
        );
        let string = Token::new(
            TokenType::Literal,
            0,
            1,
            Some(TokenLiteral::String {
                is_terminated: false,
                has_invalid_escape: false,
            }),
        );
        let tokens = HashSet::from([character, string]);

        assert_ne!(character, string);
        assert_eq!(tokens.len(), 2);
    }
}
