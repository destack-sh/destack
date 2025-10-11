use std::str::FromStr;

/// A contextual keyword.
#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    /// Visibility modifier
    Public,
    /// Visibility modifier (reserved)
    Protected,
    /// Visibility modifier (reserved)
    Internal,
    /// Visibility modifier (reserved)
    Private,
    /// Import (reserved).
    Import,
    /// Export (reserved).
    Export,
    /// Refer to the containing instance type / value.
    Self_,
    /// Refer to the containing instance type / value.
    This,
    /// Declare a Module (inline).
    Module,
    /// Declare a tuple (reserved).
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
    /// Where assertion.
    Where,
    /// Alias or cast an item.
    As,
    /// Let expression.
    Let,
    /// Var expression.
    Var,
    /// Constant modifier (alias).
    Const,
    /// Mutability modifier (alias).
    Mut,
    /// Static modifier (reserved).
    Static,
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
    /// Finally expression (reserved).
    Finally,
    /// Async expression (reserved).
    Async,
    /// Await expression (reserved).
    Await,
    /// New expression (reserved).
    New,
    /// Dynamic expression (reserved).
    Dynamic,
}

impl Keyword {
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Keyword::Public => "public",
            Keyword::Private => "private",
            Keyword::Protected => "protected",
            Keyword::Internal => "internal",
            Keyword::Import => "import",
            Keyword::Export => "export",
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
            Keyword::Where => "where",
            Keyword::As => "as",
            Keyword::Let => "let",
            Keyword::Var => "var",
            Keyword::Const => "const",
            Keyword::Mut => "mut",
            Keyword::Static => "static",
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
            Keyword::Finally => "finally",
            Keyword::Async => "async",
            Keyword::Await => "await",
            Keyword::New => "new",
            Keyword::Dynamic => "dynamic",
        }
    }
}

impl FromStr for Keyword {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "public" => Ok(Keyword::Public),
            "private" => Ok(Keyword::Private),
            "protected" => Ok(Keyword::Protected),
            "internal" => Ok(Keyword::Internal),
            "import" => Ok(Keyword::Import),
            "export" => Ok(Keyword::Export),
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
            "where" => Ok(Keyword::Where),
            "as" => Ok(Keyword::As),
            "let" => Ok(Keyword::Let),
            "var" => Ok(Keyword::Var),
            "const" => Ok(Keyword::Const),
            "mut" => Ok(Keyword::Mut),
            "static" => Ok(Keyword::Static),
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
            "finally" => Ok(Keyword::Finally),
            "async" => Ok(Keyword::Async),
            "await" => Ok(Keyword::Await),
            "new" => Ok(Keyword::New),
            "dynamic" => Ok(Keyword::Dynamic),
            _ => Err(()),
        }
    }
}
