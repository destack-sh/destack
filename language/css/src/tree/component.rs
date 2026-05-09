use crate::{Node, NodeType};
use destack_core::{StringId, StringPool};
use serde::{Deserialize, Serialize};

/// One standalone CSS component fragment root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentFragment {
    /// The component values in authored order.
    pub value: ComponentValueList,
}

impl Node for ComponentFragment {
    const TYPE: NodeType = NodeType::ComponentFragment;
}

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
    pub name: StringId,
    /// The url resource payload when this function owns one rewriteable resource.
    pub url_resource: Option<UrlResource>,
    /// The nested function argument values.
    pub arguments: ComponentValueList,
}

impl Function {
    /// Return whether this function name equals one expected value.
    pub fn name_eq(&self, strings: &StringPool, expected: &str) -> bool {
        strings.get(self.name) == expected
    }
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
    Ident(StringId),
    /// One at-keyword token.
    AtKeyword(StringId),
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
    UnquotedUrl {
        /// The unquoted url value.
        value: String,
        /// The url resource payload when this token owns one rewriteable resource.
        url_resource: Option<UrlResource>,
    },
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

/// One rewriteable CSS url resource payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UrlResource {
    /// The stable authored resource id within this stylesheet.
    pub id: u32,
    /// The path portion of the authored value.
    pub path: String,
    /// The suffix portion of the authored value.
    pub suffix: String,
    /// Whether this value should stay untouched.
    pub is_external: bool,
}

impl UrlResource {
    /// Create one url resource from one authored specifier.
    pub fn new(id: u32, specifier: &str) -> Self {
        let (path, suffix) = Self::split_specifier(specifier);

        Self {
            id,
            path: path.to_string(),
            suffix: suffix.to_string(),
            is_external: Self::is_external_specifier(specifier),
        }
    }

    /// Return whether one authored specifier should stay untouched.
    pub(crate) fn is_external_specifier(specifier: &str) -> bool {
        let specifier = specifier.trim();

        if specifier.is_empty() {
            return true;
        }

        specifier.starts_with('#')
            || specifier.starts_with("http://")
            || specifier.starts_with("https://")
            || specifier.starts_with("//")
            || specifier.starts_with("data:")
    }

    /// Split one authored specifier into path and suffix.
    pub(crate) fn split_specifier(specifier: &str) -> (&str, &str) {
        let suffix_start = specifier.find(['?', '#']).unwrap_or(specifier.len());

        specifier.split_at(suffix_start)
    }
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
    pub unit: StringId,
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
