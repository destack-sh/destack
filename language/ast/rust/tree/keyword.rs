use std::str::FromStr;

/// A contextual keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    /// Visibility modifier (reserved)
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
    /// Refer to the containing instance type.
    Self_,
    /// Refer to the containing instance type (alias to `self`).
    This,
    /// Super expression (reserved).
    Super,
    /// Declare a Module.
    Module,
    /// Declare a tuple (reserved).
    Tuple,
    /// Declare a type.
    Type,
    /// Declare a Struct.
    Struct,
    /// Declare a Class (alias to struct).
    Class,
    /// Declare an Enum.
    Enum,
    /// Declare a Union.
    Union,
    /// Declare a Trait.
    Trait,
    /// Declare an Interface (alias to trait).
    Interface,
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
    /// Constant modifier.
    Const,
    /// Mutability modifier (alias).
    Mut,
    /// Static modifier (reserved).
    Static,
    /// Final modifier (reserved).  
    Final,
    /// Do expression (reserved).
    Do,
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
    /// Switch expression (reserved).
    Switch,
    /// Case expression (reserved).
    Case,
    /// Try expression.
    Try,
    /// Catch expression.
    Catch,
    /// Throw expression (reserved).
    Throw,
    /// Finally expression (reserved).
    Finally,
    /// Async expression (reserved).
    Async,
    /// Await expression (reserved).
    Await,
    /// Unsafe expression (reserved).
    Unsafe,
    /// Move expression (reserved).
    Move,
    /// New expression (reserved).
    New,
    /// Constructor (reserved).
    Constructor,
    /// Dynamic expression (reserved).
    Dynamic,
    /// Virtual expression (reserved).
    Virtual,
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
            Keyword::Super => "super",
            Keyword::Module => "module",
            Keyword::Tuple => "tuple",
            Keyword::Type => "type",
            Keyword::Struct => "struct",
            Keyword::Class => "class",
            Keyword::Interface => "interface",
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
            Keyword::Final => "final",
            Keyword::Do => "do",
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
            Keyword::Switch => "switch",
            Keyword::Case => "case",
            Keyword::Try => "try",
            Keyword::Catch => "catch",
            Keyword::Throw => "throw",
            Keyword::Finally => "finally",
            Keyword::Async => "async",
            Keyword::Await => "await",
            Keyword::Unsafe => "unsafe",
            Keyword::Move => "move",
            Keyword::New => "new",
            Keyword::Constructor => "constructor",
            Keyword::Dynamic => "dynamic",
            Keyword::Virtual => "virtual",
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
            "super" => Ok(Keyword::Super),
            "module" => Ok(Keyword::Module),
            "type" => Ok(Keyword::Type),
            "struct" => Ok(Keyword::Struct),
            "class" => Ok(Keyword::Class),
            "interface" => Ok(Keyword::Interface),
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
            "final" => Ok(Keyword::Final),
            "do" => Ok(Keyword::Do),
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
            "switch" => Ok(Keyword::Switch),
            "case" => Ok(Keyword::Case),
            "try" => Ok(Keyword::Try),
            "catch" => Ok(Keyword::Catch),
            "throw" => Ok(Keyword::Throw),
            "finally" => Ok(Keyword::Finally),
            "async" => Ok(Keyword::Async),
            "await" => Ok(Keyword::Await),
            "unsafe" => Ok(Keyword::Unsafe),
            "move" => Ok(Keyword::Move),
            "new" => Ok(Keyword::New),
            "constructor" => Ok(Keyword::Constructor),
            "dynamic" => Ok(Keyword::Dynamic),
            "virtual" => Ok(Keyword::Virtual),
            _ => Err(()),
        }
    }
}
