use destack_serde::Reflect;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

const KEYWORD_MAX: u8 = Keyword::With as u8;

/// A Keyword in the language.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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
    /// Exclusive borrow modifier.
    Exclusive,
    /// Local placement modifier.
    Local,
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
    This,
    /// Super expression (reserved).
    Super,

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
    /// Undefined literal.
    Undefined,
    /// Keyof expression.
    Keyof,
    /// Infer expression.
    Infer,
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

    // ------------------------------------------------------------
    // Branching
    // ------------------------------------------------------------
    /// Conditional expression.
    If,
    /// Conditional expression.
    Else,
    /// Match expression.
    Match,
    /// Switch statement.
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

    // ------------------------------------------------------------
    // Errors
    // ------------------------------------------------------------
    /// Try expression.
    Try,
    /// Catch expression.
    Catch,
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
    /// With expression to declare use of items for a scope.
    With,
}

impl Keyword {
    /// Convert a dense keyword code into a keyword.
    #[inline]
    pub fn from_code(code: u8) -> Option<Self> {
        if code > KEYWORD_MAX {
            return None;
        }

        // keywords are a dense repr(u8) enum from 0 through KEYWORD_MAX
        Some(unsafe { std::mem::transmute::<u8, Keyword>(code) })
    }

    /// Return this keyword as its dense code.
    #[inline]
    pub const fn code(self) -> u8 {
        self as u8
    }

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
            Keyword::Local => "local",
            Keyword::Shared => "shared",
            Keyword::Static => "static",
            Keyword::Final => "final",
            Keyword::Virtual => "virtual",
            Keyword::Accessor => "accessor",
            Keyword::Default => "default",

            // context
            Keyword::This => "this",
            Keyword::Super => "super",

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
            Keyword::InstanceOf => "instanceof",
            Keyword::Where => "where",
            Keyword::Typeof => "typeof",
            Keyword::Void => "void",
            Keyword::Null => "null",
            Keyword::Undefined => "undefined",
            Keyword::Keyof => "keyof",
            Keyword::Infer => "infer",
            Keyword::Never => "never",
            Keyword::As => "as",
            Keyword::Is => "is",
            Keyword::In => "in",
            Keyword::Of => "of",
            Keyword::Using => "using",

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

            // errors
            Keyword::Try => "try",
            Keyword::Catch => "catch",
            Keyword::Finally => "finally",

            // async & dispatch
            Keyword::Async => "async",
            Keyword::Await => "await",
            Keyword::Get => "get",
            Keyword::Set => "set",
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
            "local" => Ok(Keyword::Local),
            "shared" => Ok(Keyword::Shared),
            "static" => Ok(Keyword::Static),
            "final" => Ok(Keyword::Final),
            "virtual" => Ok(Keyword::Virtual),
            "accessor" => Ok(Keyword::Accessor),
            "default" => Ok(Keyword::Default),

            // context
            "this" => Ok(Keyword::This),
            "super" => Ok(Keyword::Super),

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
            "instanceof" => Ok(Keyword::InstanceOf),
            "where" => Ok(Keyword::Where),
            "typeof" => Ok(Keyword::Typeof),
            "void" => Ok(Keyword::Void),
            "null" => Ok(Keyword::Null),
            "undefined" => Ok(Keyword::Undefined),
            "keyof" => Ok(Keyword::Keyof),
            "infer" => Ok(Keyword::Infer),
            "never" => Ok(Keyword::Never),
            "as" => Ok(Keyword::As),
            "is" => Ok(Keyword::Is),
            "in" => Ok(Keyword::In),
            "of" => Ok(Keyword::Of),
            "using" => Ok(Keyword::Using),

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

            // errors
            "try" => Ok(Keyword::Try),
            "catch" => Ok(Keyword::Catch),
            "finally" => Ok(Keyword::Finally),

            // async & dispatch
            "async" => Ok(Keyword::Async),
            "await" => Ok(Keyword::Await),
            "get" => Ok(Keyword::Get),
            "set" => Ok(Keyword::Set),
            "with" => Ok(Keyword::With),

            _ => Err(()),
        }
    }
}
