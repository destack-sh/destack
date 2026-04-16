use std::str::FromStr;

/// A JS/TS keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    // ------------------------------------------------------------
    // Modifiers
    // ------------------------------------------------------------
    Public,
    Protected,
    Private,
    Readonly,
    Static,
    Abstract,
    Accessor,
    Declare,
    Override,

    // ------------------------------------------------------------
    // Context
    // ------------------------------------------------------------
    This,
    Super,

    // ------------------------------------------------------------
    // Dependencies / Modules
    // ------------------------------------------------------------
    Import,
    Export,
    From,
    Default,
    As,

    // ------------------------------------------------------------
    // Declarations
    // ------------------------------------------------------------
    Const,
    Let,
    Var,
    Comptime,
    Class,
    Function,
    Enum,
    Interface,
    Type,
    Namespace,
    Module,
    Global,
    New,
    Delete,

    // ------------------------------------------------------------
    // Typing (JS/TS operators + TS type/contextual keywords)
    // ------------------------------------------------------------
    Extends,
    Implements,
    Satisfies,
    Asserts,
    InstanceOf,
    Typeof,
    Keyof,
    Infer,
    Is,
    In,
    Of,
    Void,
    Any,
    Unknown,
    Never,
    Boolean,
    Number,
    String,
    Symbol,
    Bigint,
    Object,
    Undefined,

    // ------------------------------------------------------------
    // Branching
    // ------------------------------------------------------------
    If,
    Else,
    Switch,
    Case,

    // ------------------------------------------------------------
    // Loops
    // ------------------------------------------------------------
    Do,
    While,
    For,

    // ------------------------------------------------------------
    // Flow control / statements
    // ------------------------------------------------------------
    Break,
    Continue,
    Return,
    Yield,
    With,
    Debugger,
    Using,

    // ------------------------------------------------------------
    // Errors
    // ------------------------------------------------------------
    Try,
    Catch,
    Throw,
    Finally,

    // ------------------------------------------------------------
    // Async & dispatch
    // ------------------------------------------------------------
    Async,
    Await,
    Get,
    Constructor,
    Set,
}

impl Keyword {
    pub const fn is_control(&self) -> bool {
        matches!(
            self,
            Keyword::Break
                | Keyword::Continue
                | Keyword::Return
                | Keyword::Yield
                | Keyword::If
                | Keyword::Try
                | Keyword::Catch
                | Keyword::Throw
                | Keyword::Finally
                | Keyword::Async
                | Keyword::Await
                | Keyword::Get
                | Keyword::Constructor
                | Keyword::Set
                | Keyword::Do
                | Keyword::While
                | Keyword::For
                | Keyword::Switch
                | Keyword::Case
                | Keyword::With
                | Keyword::Debugger
        )
    }

    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            // modifiers
            Keyword::Public => "public",
            Keyword::Protected => "protected",
            Keyword::Private => "private",
            Keyword::Readonly => "readonly",
            Keyword::Static => "static",
            Keyword::Abstract => "abstract",
            Keyword::Accessor => "accessor",
            Keyword::Declare => "declare",
            Keyword::Override => "override",

            // context
            Keyword::This => "this",
            Keyword::Super => "super",

            // modules
            Keyword::Import => "import",
            Keyword::Export => "export",
            Keyword::From => "from",
            Keyword::Default => "default",
            Keyword::As => "as",

            // declarations
            Keyword::Const => "const",
            Keyword::Let => "let",
            Keyword::Var => "var",
            Keyword::Comptime => "comptime",
            Keyword::Class => "class",
            Keyword::Function => "function",
            Keyword::Enum => "enum",
            Keyword::Interface => "interface",
            Keyword::Type => "type",
            Keyword::Namespace => "namespace",
            Keyword::Module => "module",
            Keyword::Global => "global",
            Keyword::New => "new",
            Keyword::Delete => "delete",

            // typing
            Keyword::Extends => "extends",
            Keyword::Implements => "implements",
            Keyword::Satisfies => "satisfies",
            Keyword::Asserts => "asserts",
            Keyword::InstanceOf => "instanceof",
            Keyword::Typeof => "typeof",
            Keyword::Keyof => "keyof",
            Keyword::Infer => "infer",
            Keyword::Is => "is",
            Keyword::In => "in",
            Keyword::Of => "of",
            Keyword::Void => "void",
            Keyword::Any => "any",
            Keyword::Unknown => "unknown",
            Keyword::Never => "never",
            Keyword::Boolean => "boolean",
            Keyword::Number => "number",
            Keyword::String => "string",
            Keyword::Symbol => "symbol",
            Keyword::Bigint => "bigint",
            Keyword::Object => "object",
            Keyword::Undefined => "undefined",

            // branching
            Keyword::If => "if",
            Keyword::Else => "else",
            Keyword::Switch => "switch",
            Keyword::Case => "case",

            // loops
            Keyword::Do => "do",
            Keyword::While => "while",
            Keyword::For => "for",

            // flow control
            Keyword::Break => "break",
            Keyword::Continue => "continue",
            Keyword::Return => "return",
            Keyword::Yield => "yield",
            Keyword::With => "with",
            Keyword::Debugger => "debugger",
            Keyword::Using => "using",

            // errors
            Keyword::Try => "try",
            Keyword::Catch => "catch",
            Keyword::Throw => "throw",
            Keyword::Finally => "finally",

            // async & dispatch
            Keyword::Async => "async",
            Keyword::Await => "await",
            Keyword::Get => "get",
            Keyword::Constructor => "constructor",
            Keyword::Set => "set",
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
            "private" => Ok(Keyword::Private),
            "readonly" => Ok(Keyword::Readonly),
            "static" => Ok(Keyword::Static),
            "abstract" => Ok(Keyword::Abstract),
            "accessor" => Ok(Keyword::Accessor),
            "declare" => Ok(Keyword::Declare),
            "override" => Ok(Keyword::Override),

            // context
            "this" => Ok(Keyword::This),
            "super" => Ok(Keyword::Super),

            // modules
            "import" => Ok(Keyword::Import),
            "export" => Ok(Keyword::Export),
            "from" => Ok(Keyword::From),
            "default" => Ok(Keyword::Default),
            "as" => Ok(Keyword::As),

            // declarations
            "const" => Ok(Keyword::Const),
            "let" => Ok(Keyword::Let),
            "var" => Ok(Keyword::Var),
            "comptime" => Ok(Keyword::Comptime),
            "class" => Ok(Keyword::Class),
            "function" => Ok(Keyword::Function),
            "enum" => Ok(Keyword::Enum),
            "interface" => Ok(Keyword::Interface),
            "type" => Ok(Keyword::Type),
            "namespace" => Ok(Keyword::Namespace),
            "module" => Ok(Keyword::Module),
            "global" => Ok(Keyword::Global),
            "new" => Ok(Keyword::New),
            "delete" => Ok(Keyword::Delete),

            // typing
            "extends" => Ok(Keyword::Extends),
            "implements" => Ok(Keyword::Implements),
            "satisfies" => Ok(Keyword::Satisfies),
            "asserts" => Ok(Keyword::Asserts),
            "instanceof" => Ok(Keyword::InstanceOf),
            "typeof" => Ok(Keyword::Typeof),
            "keyof" => Ok(Keyword::Keyof),
            "infer" => Ok(Keyword::Infer),
            "is" => Ok(Keyword::Is),
            "in" => Ok(Keyword::In),
            "of" => Ok(Keyword::Of),
            "void" => Ok(Keyword::Void),
            "any" => Ok(Keyword::Any),
            "unknown" => Ok(Keyword::Unknown),
            "never" => Ok(Keyword::Never),
            "boolean" => Ok(Keyword::Boolean),
            "number" => Ok(Keyword::Number),
            "string" => Ok(Keyword::String),
            "symbol" => Ok(Keyword::Symbol),
            "bigint" => Ok(Keyword::Bigint),
            "object" => Ok(Keyword::Object),
            "undefined" => Ok(Keyword::Undefined),

            // branching
            "if" => Ok(Keyword::If),
            "else" => Ok(Keyword::Else),
            "switch" => Ok(Keyword::Switch),
            "case" => Ok(Keyword::Case),

            // loops
            "do" => Ok(Keyword::Do),
            "while" => Ok(Keyword::While),
            "for" => Ok(Keyword::For),

            // flow control
            "break" => Ok(Keyword::Break),
            "continue" => Ok(Keyword::Continue),
            "return" => Ok(Keyword::Return),
            "yield" => Ok(Keyword::Yield),
            "with" => Ok(Keyword::With),
            "debugger" => Ok(Keyword::Debugger),
            "using" => Ok(Keyword::Using),

            // errors
            "try" => Ok(Keyword::Try),
            "catch" => Ok(Keyword::Catch),
            "throw" => Ok(Keyword::Throw),
            "finally" => Ok(Keyword::Finally),

            // async & dispatch
            "async" => Ok(Keyword::Async),
            "await" => Ok(Keyword::Await),
            "get" => Ok(Keyword::Get),
            "constructor" => Ok(Keyword::Constructor),
            "set" => Ok(Keyword::Set),

            _ => Err(()),
        }
    }
}
