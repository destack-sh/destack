use destack_source::Span;
use serde::{Deserialize, Serialize};

/// Token type for MIR text format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TokenType {
    // keywords
    /// `extern`
    Extern,
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
    /// `call`
    Call,
    /// `invoke`
    Invoke,
    /// `throw`
    Throw,
    /// `trap.abort`, `trap.panic`
    Trap,
    /// `unreachable`
    Unreachable,
    /// `tailCall`
    TailCall,
    /// `call.indirect`
    CallIndirect,
    /// `invoke.indirect`
    InvokeIndirect,
    /// `tailCall.indirect`
    TailCallIndirect,
    /// `call.virtual`
    CallVirtual,
    /// `invoke.virtual`
    InvokeVirtual,
    /// `tailCall.virtual`
    TailCallVirtual,
    /// `call.interface`
    CallInterface,
    /// `invoke.interface`
    InvokeInterface,
    /// `tailCall.interface`
    TailCallInterface,
    /// `catch`
    Catch,

    // type keywords
    /// `void`
    Void,
    /// `boolean`
    Boolean,
    /// `ref`
    Ref,
    /// `ref?`
    RefNullable,
    /// `vector`
    Vector,
    /// `tensor`
    Tensor,
    /// `tensorView`
    TensorView,
    /// `tensorView?`
    TensorViewNullable,
    /// `space`
    AddressSpace,
    /// `struct`
    Struct,
    /// `newtype`
    Newtype,

    // symbols
    /// `@`
    At,
    /// `#`
    Hash,
    /// `(`
    OpenParen,
    /// `)`
    CloseParen,
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
    /// `=`
    Equals,
    /// `->`
    Arrow,
    /// `=>`
    FatArrow,

    // literals and identifiers
    /// Identifier (instruction names, etc.)
    Identifier,
    /// Value reference: `v0`, `v1`, etc.
    Value,
    /// Block reference: `b0`, `b1`, etc.
    BlockRefence,
    /// Local reference: `local0`, `local1`, etc.
    LocalReference,
    /// Function reference: `function0`, etc.
    FunctionReference,
    /// Integer literal with type suffix: `42int32`, `0uint64`
    IntLiteral,
    /// Float literal with type suffix: `3.14float32`
    FloatLiteral,
    /// Boolean literal: `true`, `false`
    BooleanLiteral,
    /// String literal: `"hello"`
    StringLiteral,
    /// Character literal: `'a'`
    CharacterLiteral,
    /// Type name: `int32`, `uint64`, `float32`, etc.
    TypeName,

    // annotations
    /// `owned`, `borrowed`, `copy`
    Ownership,
    /// `readonly`
    Readonly,
    /// `const`
    Const,

    // trivia
    /// Whitespace (space, tab)
    Whitespace,
    /// Newline
    Newline,
    /// Line comment `//...`
    Comment,

    // special
    /// End of input
    End,
    /// Unknown/error token
    Unknown,
}

impl TokenType {
    /// Whether this token is trivia (whitespace, comments).
    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            TokenType::Whitespace | TokenType::Newline | TokenType::Comment
        )
    }
}

/// One parsed MIR token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token {
    /// The token kind.
    pub ty: TokenType,
    /// The exact source span of the token text.
    pub span: Span,
    /// The start byte offset of the token text.
    pub start: usize,
}

impl Token {
    /// Create one parsed token.
    pub fn new(ty: TokenType, span: Span) -> Self {
        Self {
            ty,
            span,
            start: span.start as usize,
        }
    }

    /// Return whether this token is trivia.
    pub fn is_trivia(&self) -> bool {
        self.ty.is_trivia()
    }
}
