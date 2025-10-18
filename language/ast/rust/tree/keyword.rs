use std::str::FromStr;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    // ------------------------------------------------------------
    // Modifiers
    // ------------------------------------------------------------
    /// Visibility modifier
    Public,
    /// Visibility modifier
    Protected,
    /// Visibility modifier (reserved)
    Internal,
    /// Visibility modifier
    Private,
    /// Constant modifier.
    Const,
    /// Readonly modifier (alias).
    Readonly,
    /// Mutability modifier (alias).
    Mut,
    /// Static modifier (reserved).
    Static,
    /// Final modifier (reserved).  
    Final,
    /// Default export mode.
    Default,

    // ------------------------------------------------------------
    // Context
    // ------------------------------------------------------------
    /// Refer to the containing instance type.
    Self_,
    /// Refer to the containing instance type (alias to `self`).
    This,
    /// Super expression (reserved).
    Super,
    /// Refer to the containing package (alias).
    Package,

    // ------------------------------------------------------------
    // Dependencies
    // ------------------------------------------------------------
    /// Import an item.
    Import,
    /// Export an item.
    Export,
    /// From expression.
    From,
    /// Import an item (reserved).
    Use,
    /// With expression to declare use of items for a scope.
    With,
    /// Declare a namespace (reserved).
    Namespace,

    // ------------------------------------------------------------
    // Definitions
    // ------------------------------------------------------------
    /// Let expression.
    Let,
    /// Var expression.
    Var,
    /// Declare a Module.
    Module,
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
    /// Declare a Trait (alias to interface).
    Trait,
    /// Declare an Interface.
    Interface,
    /// Declare a Function.
    Function,
    /// Implement a type.
    Implement,
    /// Declare statement (reserved).
    Declare,
    /// New expression (reserved).
    New,
    /// Constructor (reserved).
    Constructor,

    // ------------------------------------------------------------
    // Typing
    // ------------------------------------------------------------
    /// Extends (alias).
    Extends,
    /// Implements (alias).
    Implements,
    /// Satisfies (reserved).
    Satisfies,
    /// Override (reserved).
    Override,
    /// Declare a tuple (reserved).
    Tuple,
    /// Instanceof test.
    Instanceof,
    /// Where assertion.
    Where,
    /// Typeof expression (reserved).
    Typeof,
    /// Any expression (alias).
    Any,
    /// Never expression (alias).
    Never,
    /// Alias or cast an item.
    As,
    /// Is test.
    Is,
    /// In expression.
    In,
    /// Of expression.
    Of,

    // ------------------------------------------------------------
    // Branching
    // ------------------------------------------------------------
    /// Conditional expression.
    If,
    /// Conditional expression.
    Else,
    /// Match expression.
    Match,
    /// Switch expression (reserved).
    Switch,
    /// Case expression (reserved).
    Case,

    // ------------------------------------------------------------
    // Loops
    // ------------------------------------------------------------
    /// Do expression (reserved).
    Do,
    /// Loop expression.
    While,
    /// Loop expression.
    For,
    /// Loop expression.
    Loop,

    // ------------------------------------------------------------
    // Flow control
    // ------------------------------------------------------------
    /// Break expression.
    Break,
    /// Continue expression.
    Continue,
    /// Defer expression.
    Defer,
    /// Return expression.
    Return,
    /// Yield expression (reserved).
    Yield,

    // ------------------------------------------------------------
    // Errors
    // ------------------------------------------------------------
    /// Try expression.
    Try,
    /// Catch expression.
    Catch,
    /// Throw expression (reserved).
    Throw,
    /// Finally expression (reserved).
    Finally,

    // ------------------------------------------------------------
    // Async & Dispatch
    // ------------------------------------------------------------
    /// Async expression (reserved).
    Async,
    /// Await expression (reserved).
    Await,
    /// Unsafe expression (reserved).
    Unsafe,
    /// Move expression (reserved).
    Move,
    /// Dynamic expression (reserved).
    Dynamic,
    /// Virtual expression (reserved).
    Virtual,
}

impl Keyword {
    pub const fn is_control(&self) -> bool {
        matches!(
            self,
            Keyword::Break
                | Keyword::Continue
                | Keyword::Defer
                | Keyword::Return
                | Keyword::Yield
                | Keyword::If
                | Keyword::Try
                | Keyword::Catch
                | Keyword::Throw
                | Keyword::Finally
                | Keyword::Async
                | Keyword::Await
                | Keyword::Unsafe
                | Keyword::Move
                | Keyword::Dynamic
                | Keyword::Virtual
                | Keyword::Do
                | Keyword::While
                | Keyword::For
                | Keyword::Loop
                | Keyword::Match
                | Keyword::Switch
                | Keyword::Case
        )
    }

    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            // modifiers
            Keyword::Public => "public",
            Keyword::Protected => "protected",
            Keyword::Internal => "internal",
            Keyword::Private => "private",
            Keyword::Const => "const",
            Keyword::Readonly => "readonly",
            Keyword::Mut => "mut",
            Keyword::Static => "static",
            Keyword::Final => "final",
            Keyword::Default => "default",

            // context
            Keyword::Self_ => "self",
            Keyword::This => "this",
            Keyword::Super => "super",
            Keyword::Package => "package",

            // dependencies
            Keyword::Import => "import",
            Keyword::Export => "export",
            Keyword::From => "from",
            Keyword::Use => "use",
            Keyword::With => "with",
            Keyword::Namespace => "namespace",

            // definitions
            Keyword::Let => "let",
            Keyword::Var => "var",
            Keyword::Module => "module",
            Keyword::Type => "type",
            Keyword::Struct => "struct",
            Keyword::Class => "class",
            Keyword::Enum => "enum",
            Keyword::Union => "union",
            Keyword::Trait => "trait",
            Keyword::Interface => "interface",
            Keyword::Function => "function",
            Keyword::Implement => "implement",
            Keyword::Declare => "declare",
            Keyword::New => "new",
            Keyword::Constructor => "constructor",

            // typing
            Keyword::Extends => "extends",
            Keyword::Implements => "implements",
            Keyword::Satisfies => "satisfies",
            Keyword::Override => "override",
            Keyword::Tuple => "tuple",
            Keyword::Instanceof => "instanceof",
            Keyword::Where => "where",
            Keyword::Typeof => "typeof",
            Keyword::Any => "any",
            Keyword::Never => "never",
            Keyword::As => "as",
            Keyword::Is => "is",
            Keyword::In => "in",
            Keyword::Of => "of",

            // branching
            Keyword::If => "if",
            Keyword::Else => "else",
            Keyword::Match => "match",
            Keyword::Switch => "switch",
            Keyword::Case => "case",

            // loops
            Keyword::Do => "do",
            Keyword::While => "while",
            Keyword::For => "for",
            Keyword::Loop => "loop",

            // flow control
            Keyword::Break => "break",
            Keyword::Continue => "continue",
            Keyword::Defer => "defer",
            Keyword::Return => "return",
            Keyword::Yield => "yield",

            // errors
            Keyword::Try => "try",
            Keyword::Catch => "catch",
            Keyword::Throw => "throw",
            Keyword::Finally => "finally",

            // async & dispatch
            Keyword::Async => "async",
            Keyword::Await => "await",
            Keyword::Unsafe => "unsafe",
            Keyword::Move => "move",
            Keyword::Dynamic => "dynamic",
            Keyword::Virtual => "virtual",
        }
    }
}

impl FromStr for Keyword {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            // modifiers
            "public" => Ok(Keyword::Public),
            "protected" => Ok(Keyword::Protected),
            "internal" => Ok(Keyword::Internal),
            "private" => Ok(Keyword::Private),
            "const" => Ok(Keyword::Const),
            "readonly" => Ok(Keyword::Readonly),
            "mut" => Ok(Keyword::Mut),
            "static" => Ok(Keyword::Static),
            "final" => Ok(Keyword::Final),
            "default" => Ok(Keyword::Default),

            // context
            "self" => Ok(Keyword::Self_),
            "this" => Ok(Keyword::This),
            "super" => Ok(Keyword::Super),
            "package" => Ok(Keyword::Package),

            // dependencies
            "import" => Ok(Keyword::Import),
            "export" => Ok(Keyword::Export),
            "from" => Ok(Keyword::From),
            "use" => Ok(Keyword::Use),
            "with" => Ok(Keyword::With),
            "namespace" => Ok(Keyword::Namespace),

            // definitions
            "let" => Ok(Keyword::Let),
            "var" => Ok(Keyword::Var),
            "module" => Ok(Keyword::Module),
            "type" => Ok(Keyword::Type),
            "struct" => Ok(Keyword::Struct),
            "class" => Ok(Keyword::Class),
            "enum" => Ok(Keyword::Enum),
            "union" => Ok(Keyword::Union),
            "trait" => Ok(Keyword::Trait),
            "interface" => Ok(Keyword::Interface),
            "function" => Ok(Keyword::Function),
            "implement" => Ok(Keyword::Implement),
            "declare" => Ok(Keyword::Declare),
            "new" => Ok(Keyword::New),
            "constructor" => Ok(Keyword::Constructor),

            // typing
            "extends" => Ok(Keyword::Extends),
            "implements" => Ok(Keyword::Implements),
            "satisfies" => Ok(Keyword::Satisfies),
            "override" => Ok(Keyword::Override),
            "tuple" => Ok(Keyword::Tuple),
            "instanceof" => Ok(Keyword::Instanceof),
            "where" => Ok(Keyword::Where),
            "typeof" => Ok(Keyword::Typeof),
            "any" => Ok(Keyword::Any),
            "never" => Ok(Keyword::Never),
            "as" => Ok(Keyword::As),
            "is" => Ok(Keyword::Is),
            "in" => Ok(Keyword::In),
            "of" => Ok(Keyword::Of),

            // branching
            "if" => Ok(Keyword::If),
            "else" => Ok(Keyword::Else),
            "match" => Ok(Keyword::Match),
            "switch" => Ok(Keyword::Switch),
            "case" => Ok(Keyword::Case),

            // loops
            "do" => Ok(Keyword::Do),
            "while" => Ok(Keyword::While),
            "for" => Ok(Keyword::For),
            "loop" => Ok(Keyword::Loop),

            // flow control
            "break" => Ok(Keyword::Break),
            "continue" => Ok(Keyword::Continue),
            "defer" => Ok(Keyword::Defer),
            "return" => Ok(Keyword::Return),
            "yield" => Ok(Keyword::Yield),

            // errors
            "try" => Ok(Keyword::Try),
            "catch" => Ok(Keyword::Catch),
            "throw" => Ok(Keyword::Throw),
            "finally" => Ok(Keyword::Finally),

            // async & dispatch
            "async" => Ok(Keyword::Async),
            "await" => Ok(Keyword::Await),
            "unsafe" => Ok(Keyword::Unsafe),
            "move" => Ok(Keyword::Move),
            "dynamic" => Ok(Keyword::Dynamic),
            "virtual" => Ok(Keyword::Virtual),

            _ => Err(()),
        }
    }
}
