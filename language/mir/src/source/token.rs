use destack_source::Span;
use serde::{Deserialize, Serialize};

/// One lexical MIR token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Token {
    /// The token type.
    pub ty: TokenType,
    /// The exact source span.
    pub span: Span,
    /// The source start byte.
    pub start: usize,
}

impl Token {
    /// Create one token.
    #[inline]
    pub const fn new(ty: TokenType, span: Span) -> Self {
        Self {
            ty,
            span,
            start: span.start as usize,
        }
    }

    /// Return the source start byte.
    #[inline]
    pub const fn start(self) -> usize {
        self.start
    }

    /// Return whether this token is trivia.
    #[inline]
    pub const fn is_trivia(self) -> bool {
        matches!(
            self.ty,
            TokenType::Whitespace | TokenType::Newline | TokenType::Comment
        )
    }
}

/// MIR token type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TokenType {
    /// Identifier text.
    Identifier,
    /// Integer literal text.
    Integer,
    /// Floating point literal text.
    Float,
    /// String literal text.
    String,
    /// Character literal text.
    Character,
    /// Non-newline whitespace.
    Whitespace,
    /// Newline sequence.
    Newline,
    /// Line comment.
    Comment,
    /// End of source.
    End,
    /// Unknown token.
    Unknown,
    /// `@`
    At,
    /// `#`
    Hash,
    /// `(`
    OpenParenthesis,
    /// `)`
    CloseParenthesis,
    /// `{`
    OpenBrace,
    /// `}`
    CloseBrace,
    /// `[`
    OpenBracket,
    /// `]`
    CloseBracket,
    /// `<`
    LessThan,
    /// `>`
    GreaterThan,
    /// `:`
    Colon,
    /// `;`
    Semicolon,
    /// `,`
    Comma,
    /// `?`
    Question,
    /// `=`
    Equal,
    /// `->`
    Arrow,
    /// `=>`
    FatArrow,
    /// `external`
    External,
    /// `export`
    Export,
    /// `function`
    Function,
    /// `global`
    Global,
    /// `type`
    Type,
    /// `block`
    Block,
    /// `local`
    Local,
    /// `return`
    Return,
    /// `jump`
    Jump,
    /// `branch`
    Branch,
    /// `check`
    Check,
    /// `switch`
    Switch,
    /// `yield`
    Yield,
    /// `panic`
    Panic,
    /// `panic.resume`
    ResumePanic,
    /// `trap.abort`
    Trap,
    /// `unreachable`
    Unreachable,
    /// `tailCall`
    TailCall,
    /// `call`
    Call,
    /// `call.indirect`
    CallIndirect,
    /// `tailCall.indirect`
    TailCallIndirect,
    /// `call.class`
    CallClass,
    /// `tailCall.class`
    TailCallClass,
    /// `call.interface`
    CallInterface,
    /// `tailCall.interface`
    TailCallInterface,
    /// `void`
    Void,
    /// `boolean`
    Boolean,
    /// `ref`
    Ref,
    /// `vector`
    Vector,
    /// `tensor`
    Tensor,
    /// `tensorView`
    TensorView,
    /// `space`
    Space,
    /// `struct`
    Struct,
    /// `newtype`
    Newtype,
    /// SSA value reference.
    Value,
    /// Block reference.
    BlockReference,
    /// Local reference.
    LocalReference,
    /// Function reference.
    FunctionReference,
    /// Boolean literal.
    BooleanLiteral,
    /// Primitive type name.
    TypeName,
    /// Ownership or copy annotation.
    Ownership,
    /// `readonly`
    Readonly,
    /// `const`
    Const,
}

impl TokenType {
    /// Return the MIR token type for one identifier.
    pub(crate) fn from_identifier(text: &str) -> Self {
        match text {
            "external" => Self::External,
            "export" => Self::Export,
            "function" => Self::Function,
            "global" => Self::Global,
            "type" => Self::Type,
            "block" => Self::Block,
            "local" => Self::Local,
            "return" => Self::Return,
            "jump" => Self::Jump,
            "branch" => Self::Branch,
            "check" => Self::Check,
            "switch" => Self::Switch,
            "yield" => Self::Yield,
            "panic" => Self::Panic,
            "panic.resume" => Self::ResumePanic,
            "trap.abort" => Self::Trap,
            "unreachable" => Self::Unreachable,
            "tailCall" => Self::TailCall,
            "call" => Self::Call,
            "call.indirect" => Self::CallIndirect,
            "tailCall.indirect" => Self::TailCallIndirect,
            "call.class" => Self::CallClass,
            "tailCall.class" => Self::TailCallClass,
            "call.interface" => Self::CallInterface,
            "tailCall.interface" => Self::TailCallInterface,
            "void" => Self::Void,
            "boolean" => Self::Boolean,
            "ref" => Self::Ref,
            "vector" => Self::Vector,
            "tensor" => Self::Tensor,
            "tensorView" => Self::TensorView,
            "space" => Self::Space,
            "struct" => Self::Struct,
            "newtype" => Self::Newtype,
            "true" | "false" => Self::BooleanLiteral,
            "owned" | "borrowed" | "copy" => Self::Ownership,
            "readonly" => Self::Readonly,
            "const" => Self::Const,
            _ if is_value_name(text) => Self::Value,
            _ if is_block_name(text) => Self::BlockReference,
            _ if is_local_name(text) => Self::LocalReference,
            _ if is_function_name(text) => Self::FunctionReference,
            _ if is_primitive_type_name(text) => Self::TypeName,
            _ => Self::Identifier,
        }
    }
}

/// Return whether text is a numeric value name.
fn is_value_name(text: &str) -> bool {
    text.strip_prefix('v').is_some_and(|rest| {
        !rest.is_empty() && rest.chars().all(|character| character.is_ascii_digit())
    })
}

/// Return whether text is a numeric block name.
fn is_block_name(text: &str) -> bool {
    text.strip_prefix('b').is_some_and(|rest| {
        !rest.is_empty() && rest.chars().all(|character| character.is_ascii_digit())
    })
}

/// Return whether text is a numeric local name.
fn is_local_name(text: &str) -> bool {
    text.strip_prefix("local").is_some_and(|rest| {
        !rest.is_empty() && rest.chars().all(|character| character.is_ascii_digit())
    })
}

/// Return whether text is a numeric function name.
fn is_function_name(text: &str) -> bool {
    text.strip_prefix("function").is_some_and(|rest| {
        !rest.is_empty() && rest.chars().all(|character| character.is_ascii_digit())
    })
}

/// Return whether text is a primitive MIR type name.
fn is_primitive_type_name(text: &str) -> bool {
    matches!(
        text,
        "int8"
            | "int16"
            | "int32"
            | "int64"
            | "int128"
            | "int256"
            | "uint8"
            | "uint16"
            | "uint32"
            | "uint64"
            | "uint128"
            | "uint256"
            | "float32"
            | "float64"
            | "isize"
            | "usize"
            | "typeDescriptor"
            | "typeId"
    )
}
