use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A Keyword in the language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Keyword {
    // ------------------------------------------------------------
    // Modifiers
    // ------------------------------------------------------------
    /// Visibility modifier
    Public,
    /// Visibility modifier
    Protected,
    /// Visibility modifier
    Private,
    /// Readonly modifier (alias).
    Readonly,
    /// Exclusive access modifier.
    Exclusive,
    /// Shared placement modifier.
    Shared,
    /// Static modifier (reserved).
    Static,
    /// Final modifier (reserved).
    Final,
    /// Virtual dispatch modifier.
    Virtual,
    /// Accessor modifier (auto-accessor).
    Accessor,
    /// Default export type.
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

    // ------------------------------------------------------------
    // Declarations
    // ------------------------------------------------------------
    /// Constant modifier.
    Const,
    /// Let expression.
    Let,
    /// Declare a type.
    Type,
    /// Declare a newtype.
    Newtype,
    /// Declare a Struct.
    Struct,
    /// Declare a Class (alias to struct).
    Class,
    /// Declare an Enum.
    Enum,
    /// Declare a Union.
    Union,
    /// Declare an Interface.
    Interface,
    /// Declare a Function.
    Function,
    /// Extend a type.
    Extension,
    /// Declare declaration.
    Declare,
    /// New expression.
    New,
    /// Constructor.
    Constructor,

    // ------------------------------------------------------------
    // Typing
    // ------------------------------------------------------------
    /// Asserts expression.
    Asserts,
    /// Extends.
    Extends,
    /// Implements.
    Implements,
    /// Satisfies.
    Satisfies,
    /// Abstract modifier.
    Abstract,
    /// Override.
    Override,
    /// Instanceof test.
    InstanceOf,
    /// Where assertion.
    Where,
    /// Typeof expression.
    Typeof,
    /// Void type.
    Void,
    /// Null literal.
    Null,
    /// Keyof expression.
    Keyof,
    /// Infer expression.
    Infer,
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
    /// Using clause for generics or type context
    Using,
    /// Provides clause for generics or type context
    Provides,
    /// Compile time evaluation.
    Comptime,

    // ------------------------------------------------------------
    // Branching
    // ------------------------------------------------------------
    /// Conditional expression.
    If,
    /// Conditional expression.
    Else,
    /// Match expression.
    Match,
    /// Switch expression.
    Switch,
    /// Case expression.
    Case,

    // ------------------------------------------------------------
    // Loops
    // ------------------------------------------------------------
    /// Do expression.
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
    /// Debugger statement.
    Debugger,
    /// Return expression.
    Return,
    /// Yield expression.
    Yield,
    /// Goto expression (reserved).
    Goto,

    // ------------------------------------------------------------
    // Errors
    // ------------------------------------------------------------
    /// Try expression.
    Try,
    /// Catch expression.
    Catch,
    /// Throw expression.
    Throw,
    /// Finally expression.
    Finally,

    // ------------------------------------------------------------
    // Async & Dispatch
    // ------------------------------------------------------------
    /// Async expression.
    Async,
    /// Await expression.
    Await,
    /// Getter function.
    Get,
    /// Setter function.
    Set,
    /// Move values.
    Move,
    /// With expression to declare use of items for a scope.
    With,
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
                | Keyword::Set
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
            Keyword::Private => "private",
            Keyword::Readonly => "readonly",
            Keyword::Exclusive => "exclusive",
            Keyword::Shared => "shared",
            Keyword::Static => "static",
            Keyword::Final => "final",
            Keyword::Virtual => "virtual",
            Keyword::Accessor => "accessor",
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

            // declarations
            Keyword::Const => "const",
            Keyword::Let => "let",
            Keyword::Type => "type",
            Keyword::Newtype => "newtype",
            Keyword::Struct => "struct",
            Keyword::Class => "class",
            Keyword::Enum => "enum",
            Keyword::Union => "union",
            Keyword::Interface => "interface",
            Keyword::Function => "function",
            Keyword::Extension => "extension",
            Keyword::Declare => "declare",
            Keyword::New => "new",
            Keyword::Constructor => "constructor",

            // typing
            Keyword::Extends => "extends",
            Keyword::Implements => "implements",
            Keyword::Satisfies => "satisfies",
            Keyword::Abstract => "abstract",
            Keyword::Override => "override",
            Keyword::Asserts => "asserts",
            Keyword::InstanceOf => "instanceof",
            Keyword::Where => "where",
            Keyword::Typeof => "typeof",
            Keyword::Void => "void",
            Keyword::Null => "null",
            Keyword::Keyof => "keyof",
            Keyword::Infer => "infer",
            Keyword::Any => "any",
            Keyword::Never => "never",
            Keyword::As => "as",
            Keyword::Is => "is",
            Keyword::In => "in",
            Keyword::Of => "of",
            Keyword::Using => "using",
            Keyword::Provides => "provides",
            Keyword::Comptime => "comptime",

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
            Keyword::Debugger => "debugger",
            Keyword::Return => "return",
            Keyword::Yield => "yield",
            Keyword::Goto => "goto",

            // errors
            Keyword::Try => "try",
            Keyword::Catch => "catch",
            Keyword::Throw => "throw",
            Keyword::Finally => "finally",

            // async & dispatch
            Keyword::Async => "async",
            Keyword::Await => "await",
            Keyword::Get => "get",
            Keyword::Set => "set",
            Keyword::Move => "move",
            Keyword::With => "with",
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
            "exclusive" => Ok(Keyword::Exclusive),
            "shared" => Ok(Keyword::Shared),
            "static" => Ok(Keyword::Static),
            "final" => Ok(Keyword::Final),
            "virtual" => Ok(Keyword::Virtual),
            "accessor" => Ok(Keyword::Accessor),
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

            // declarations
            "const" => Ok(Keyword::Const),
            "let" => Ok(Keyword::Let),
            "type" => Ok(Keyword::Type),
            "newtype" => Ok(Keyword::Newtype),
            "struct" => Ok(Keyword::Struct),
            "class" => Ok(Keyword::Class),
            "enum" => Ok(Keyword::Enum),
            "union" => Ok(Keyword::Union),
            "interface" => Ok(Keyword::Interface),
            "function" => Ok(Keyword::Function),
            "extension" => Ok(Keyword::Extension),
            "declare" => Ok(Keyword::Declare),
            "new" => Ok(Keyword::New),
            "constructor" => Ok(Keyword::Constructor),

            // typing
            "extends" => Ok(Keyword::Extends),
            "implements" => Ok(Keyword::Implements),
            "satisfies" => Ok(Keyword::Satisfies),
            "abstract" => Ok(Keyword::Abstract),
            "override" => Ok(Keyword::Override),
            "asserts" => Ok(Keyword::Asserts),
            "instanceof" => Ok(Keyword::InstanceOf),
            "where" => Ok(Keyword::Where),
            "typeof" => Ok(Keyword::Typeof),
            "void" => Ok(Keyword::Void),
            "null" => Ok(Keyword::Null),
            "keyof" => Ok(Keyword::Keyof),
            "infer" => Ok(Keyword::Infer),
            "any" => Ok(Keyword::Any),
            "never" => Ok(Keyword::Never),
            "as" => Ok(Keyword::As),
            "is" => Ok(Keyword::Is),
            "in" => Ok(Keyword::In),
            "of" => Ok(Keyword::Of),
            "using" => Ok(Keyword::Using),
            "provides" => Ok(Keyword::Provides),
            "comptime" => Ok(Keyword::Comptime),

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
            "debugger" => Ok(Keyword::Debugger),
            "return" => Ok(Keyword::Return),
            "yield" => Ok(Keyword::Yield),
            "goto" => Ok(Keyword::Goto),

            // errors
            "try" => Ok(Keyword::Try),
            "catch" => Ok(Keyword::Catch),
            "throw" => Ok(Keyword::Throw),
            "finally" => Ok(Keyword::Finally),

            // async & dispatch
            "async" => Ok(Keyword::Async),
            "await" => Ok(Keyword::Await),
            "get" => Ok(Keyword::Get),
            "set" => Ok(Keyword::Set),
            "move" => Ok(Keyword::Move),
            "with" => Ok(Keyword::With),

            _ => Err(()),
        }
    }
}
