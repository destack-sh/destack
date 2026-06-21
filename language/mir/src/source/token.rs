use destack_serde::Schema;
use destack_source::Span;
use serde::{Deserialize, Serialize};

/// One lexical MIR token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Schema)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Schema)]
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
    /// `|`
    Pipe,
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
    /// `unwind.resume`
    UnwindResume,
    /// `trap.abort`
    Trap,
    /// `unreachable`
    Unreachable,
    /// `tail.call`
    TailCall,
    /// `call`
    Call,
    /// `call.indirect`
    CallIndirect,
    /// `tail.call.indirect`
    TailCallIndirect,
    /// `call.virtual`
    CallVirtual,
    /// `tail.call.virtual`
    TailCallVirtual,
    /// `call.dynamic`
    CallDynamic,
    /// `tail.call.dynamic`
    TailCallDynamic,
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
            "unwind.resume" => Self::UnwindResume,
            "trap.abort" => Self::Trap,
            "unreachable" => Self::Unreachable,
            "tail.call" => Self::TailCall,
            "call" => Self::Call,
            "call.indirect" => Self::CallIndirect,
            "tail.call.indirect" => Self::TailCallIndirect,
            "call.virtual" => Self::CallVirtual,
            "tail.call.virtual" => Self::TailCallVirtual,
            "call.dynamic" => Self::CallDynamic,
            "tail.call.dynamic" => Self::TailCallDynamic,
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
            _ if is_primitive_type_name(text) => Self::TypeName,
            _ => Self::Identifier,
        }
    }
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
            | "float16"
            | "bfloat16"
            | "float32"
            | "float64"
            | "isize"
            | "usize"
            | "typeDescriptor"
            | "typeId"
    )
}
