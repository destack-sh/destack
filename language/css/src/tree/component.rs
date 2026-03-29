use serde::{Deserialize, Serialize};

/// One generic CSS component value list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentValueList {
    /// The component values in authored order.
    pub values: Vec<ComponentValue>,
}

/// One generic CSS component value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentValue {
    /// One plain token.
    Token(Token),
    /// One function with nested component values.
    Function(Function),
    /// One simple block with nested component values.
    Block(SimpleBlock),
}

/// One CSS function component value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Function {
    /// The function name without the trailing `(`.
    pub name: String,
    /// The nested function argument values.
    pub arguments: ComponentValueList,
}

/// One CSS simple block component value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimpleBlock {
    /// The block delimiter kind.
    pub kind: BlockKind,
    /// The nested block values.
    pub value: ComponentValueList,
}

/// One CSS block delimiter kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockKind {
    /// One `(...)` block.
    Parenthesis,
    /// One `[...]` block.
    SquareBracket,
    /// One `{...}` block.
    CurlyBracket,
}

/// One generic CSS token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Token {
    /// One identifier token.
    Ident(String),
    /// One at-keyword token.
    AtKeyword(String),
    /// One hash token.
    Hash {
        /// The hash text without `#`.
        value: String,
        /// Whether this hash is an id-hash token.
        is_identifier: bool,
    },
    /// One string token without quotes.
    String(String),
    /// One unquoted url token without `url(` and `)`.
    UnquotedUrl(String),
    /// One single delimiter token.
    Delimiter(char),
    /// One number token.
    Number(Number),
    /// One percentage token.
    Percentage(Number),
    /// One dimension token.
    Dimension(Dimension),
    /// One whitespace token.
    WhiteSpace(String),
    /// One comment token without delimiters.
    Comment(String),
    /// One punctuation token.
    Symbol(Symbol),
    /// One bad url token.
    BadUrl(String),
    /// One bad string token.
    BadString(String),
}

/// One numeric CSS token payload.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Number {
    /// Whether the number has an explicit sign.
    pub has_sign: bool,
    /// The numeric value.
    pub value: f32,
    /// The integer value when it was authored without a fractional part.
    pub integer_value: Option<i32>,
}

impl Eq for Number {}

/// One dimension CSS token payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dimension {
    /// The numeric portion of the dimension.
    pub number: Number,
    /// The unit suffix.
    pub unit: String,
}

/// One symbolic CSS token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Symbol {
    /// One `:` token.
    Colon,
    /// One `;` token.
    Semicolon,
    /// One `,` token.
    Comma,
    /// One `~=` token.
    IncludeMatch,
    /// One `|=` token.
    DashMatch,
    /// One `^=` token.
    PrefixMatch,
    /// One `$=` token.
    SuffixMatch,
    /// One `*=` token.
    SubstringMatch,
    /// One `<!--` token.
    Cdo,
    /// One `-->` token.
    Cdc,
}
