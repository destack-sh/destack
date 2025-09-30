use std::str::FromStr;

/// A contextual keyword.
#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    /// Mark the following item as public (with optional qualifier)
    Public,
    /// Mark the following item as private (with optional qualifier)
    Private,
    /// Refer to the own instance.
    Self_,
    /// Refer to the own instance.
    This,
    /// Declare a Module (inline).
    Module,
    /// Declare a tuple.
    Tuple,
    /// Declare a type.
    Type,
    /// Declare a Struct.
    Struct,
    /// Declare an Enum.
    Enum,
    /// Declare a Union.
    Union,
    /// Declare a Trait.
    Trait,
    /// Declare a Function.
    Function,
    /// Implement a type.
    Implement,
    /// Use an item in this scope.
    Use,
    /// With expression to declare use of items for a scope.
    With,
    /// Alias or cast an item.
    As,
    /// Constant modifier.
    Const,
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
            Keyword::Private => "private",
            Keyword::Self_ => "self",
            Keyword::This => "this",
            Keyword::Module => "module",
            Keyword::Tuple => "tuple",
            Keyword::Type => "type",
            Keyword::Struct => "struct",
            Keyword::Enum => "enum",
            Keyword::Union => "union",
            Keyword::Trait => "trait",
            Keyword::Function => "function",
            Keyword::Implement => "implement",
            Keyword::Use => "use",
            Keyword::With => "with",
            Keyword::As => "as",
            Keyword::Const => "const",
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

impl FromStr for Keyword {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "public" => Ok(Keyword::Public),
            "private" => Ok(Keyword::Private),
            "self" => Ok(Keyword::Self_),
            "this" => Ok(Keyword::This),
            "tuple" => Ok(Keyword::Tuple),
            "module" => Ok(Keyword::Module),
            "type" => Ok(Keyword::Type),
            "struct" => Ok(Keyword::Struct),
            "enum" => Ok(Keyword::Enum),
            "union" => Ok(Keyword::Union),
            "trait" => Ok(Keyword::Trait),
            "function" => Ok(Keyword::Function),
            "implement" => Ok(Keyword::Implement),
            "use" => Ok(Keyword::Use),
            "with" => Ok(Keyword::With),
            "as" => Ok(Keyword::As),
            "const" => Ok(Keyword::Const),
            "let" => Ok(Keyword::Let),
            "var" => Ok(Keyword::Var),
            "if" => Ok(Keyword::If),
            "else" => Ok(Keyword::Else),
            "while" => Ok(Keyword::While),
            "for" => Ok(Keyword::For),
            "in" => Ok(Keyword::In),
            "loop" => Ok(Keyword::Loop),
            "break" => Ok(Keyword::Break),
            "continue" => Ok(Keyword::Continue),
            "defer" => Ok(Keyword::Defer),
            "return" => Ok(Keyword::Return),
            "match" => Ok(Keyword::Match),
            "try" => Ok(Keyword::Try),
            "catch" => Ok(Keyword::Catch),
            _ => Err(()),
        }
    }
}
