/// A contextual keyword.
#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    /// Mark the following item as public (with optional qualifier)
    Public,
    /// Refer to the own type.
    Self_,
    /// Define a Module (inline).
    Module,
    /// Define a Struct.
    Struct,
    /// Define an Enum.
    Enum,
    /// Define a Union.
    Union,
    /// Define a Trait.
    Trait,
    /// Define a Function.
    Function,
    /// Implement a type.
    Implement,
    /// Use an item in this scope.
    Use,
    /// With expression to declare use of items for a scope.
    With,
    /// Alias or cast an item.
    As,
    /// Let expression.
    Let,
    /// Var expression.
    Var,
    /// Conditional expression.
    If,
    /// Conditional expression.
    Else,
    /// Loop expression.
    While,
    /// Loop expression.
    For,
    /// Loop expression.
    In,
    /// Loop expression.
    Loop,
    /// Break expression.
    Break,
    /// Continue expression.
    Continue,
    /// Defer expression.
    Defer,
    /// Return expression.
    Return,
    /// Match expression.
    Match,
    /// Try expression.
    Try,
    /// Catch expression.
    Catch,
}

impl Keyword {
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Keyword::Public => "public",
            Keyword::Self_ => "self",
            Keyword::Module => "module",
            Keyword::Struct => "struct",
            Keyword::Enum => "enum",
            Keyword::Union => "union",
            Keyword::Trait => "trait",
            Keyword::Function => "function",
            Keyword::Implement => "implement",
            Keyword::Use => "use",
            Keyword::With => "with",
            Keyword::As => "as",
            Keyword::Let => "let",
            Keyword::Var => "var",
            Keyword::If => "if",
            Keyword::Else => "else",
            Keyword::While => "while",
            Keyword::For => "for",
            Keyword::In => "in",
            Keyword::Loop => "loop",
            Keyword::Break => "break",
            Keyword::Continue => "continue",
            Keyword::Defer => "defer",
            Keyword::Return => "return",
            Keyword::Match => "match",
            Keyword::Try => "try",
            Keyword::Catch => "catch",
        }
    }
}
