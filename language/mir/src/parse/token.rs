/// Token type for MIR text format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// `tensorRef`
    TensorReference,
    /// `tensorRef?`
    TensorReferenceNullable,
    /// `addressSpace`
    AddressSpace,
    /// `fn`
    Fn,
    /// `closure`
    Closure,
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
    BoolLiteral,
    /// String literal: `"hello"`
    StringLiteral,
    /// Character literal: `'a'`
    CharLiteral,
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

/// A token with its source text.
#[derive(Debug, Clone)]
pub struct Token<'a> {
    /// The token type.
    pub ty: TokenType,
    /// The source text of the token.
    pub text: &'a str,
    /// Start position in the source.
    pub start: usize,
}

impl<'a> Token<'a> {
    /// Create a new token.
    pub fn new(ty: TokenType, text: &'a str, start: usize) -> Self {
        Self { ty, text, start }
    }
}
