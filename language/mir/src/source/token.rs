use destack_serde::Reflect;
use destack_source::Span;
use serde::{Deserialize, Serialize};

use crate::Type;

/// One lexical MIR token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Token {
    /// The token type.
    pub ty: TokenType,
    /// The exact source span.
    pub span: Span,
}

impl Token {
    /// Create one token.
    #[inline]
    pub const fn new(ty: TokenType, span: Span) -> Self {
        Self { ty, span }
    }

    /// Return the source start byte.
    #[inline]
    pub const fn start(self) -> usize {
        self.span.start as usize
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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
    /// Regular expression literal text.
    Regex,
    /// Tick lifetime name like `'a`.
    Lifetime,
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
    /// `.`
    Dot,
    /// `|`
    Pipe,
    /// `&`
    Ampersand,
    /// `?`
    Question,
    /// `*`
    Star,
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
    /// `shared`
    Shared,
    /// `constant`
    Constant,
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
    /// `panic`
    Panic,
    /// `unwind.resume`
    UnwindResume,
    /// `abort`
    Abort,
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
    /// `tail.call.witness`
    TailCallWitness,
    /// `invoke`
    Invoke,
    /// `invoke.indirect`
    InvokeIndirect,
    /// `invoke.virtual`
    InvokeVirtual,
    /// `invoke.dynamic`
    InvokeDynamic,
    /// `invoke.witness`
    InvokeWitness,
    /// `void`
    Void,
    /// `boolean`
    Boolean,
    /// `ref`
    Ref,
    /// `vector`
    Vector,
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
    /// Return whether this token is an identifier or a keyword usable as a member name.
    pub(crate) const fn is_name(self) -> bool {
        matches!(
            self,
            Self::Identifier
                | Self::External
                | Self::Export
                | Self::Function
                | Self::Global
                | Self::Shared
                | Self::Constant
                | Self::Type
                | Self::Block
                | Self::Local
                | Self::Return
                | Self::Jump
                | Self::Branch
                | Self::Check
                | Self::Switch
                | Self::Panic
                | Self::UnwindResume
                | Self::Abort
                | Self::Unreachable
                | Self::TailCall
                | Self::Call
                | Self::CallIndirect
                | Self::TailCallIndirect
                | Self::CallVirtual
                | Self::TailCallVirtual
                | Self::CallDynamic
                | Self::TailCallDynamic
                | Self::TailCallWitness
                | Self::Invoke
                | Self::InvokeIndirect
                | Self::InvokeVirtual
                | Self::InvokeDynamic
                | Self::InvokeWitness
                | Self::Void
                | Self::Boolean
                | Self::Ref
                | Self::Vector
                | Self::Struct
                | Self::Newtype
                | Self::BooleanLiteral
                | Self::TypeName
                | Self::Ownership
                | Self::Readonly
                | Self::Const
        )
    }

    /// Return whether this token begins an invoke terminator.
    pub(crate) const fn is_invoke(self) -> bool {
        matches!(
            self,
            Self::Invoke
                | Self::InvokeIndirect
                | Self::InvokeVirtual
                | Self::InvokeDynamic
                | Self::InvokeWitness
        )
    }

    /// Return whether this token begins a tail-call terminator.
    pub(crate) const fn is_tail_call(self) -> bool {
        matches!(
            self,
            Self::TailCall
                | Self::TailCallIndirect
                | Self::TailCallVirtual
                | Self::TailCallDynamic
                | Self::TailCallWitness
        )
    }

    /// Return whether this token begins a block terminator.
    pub(crate) const fn is_terminator(self) -> bool {
        matches!(
            self,
            Self::Return
                | Self::Jump
                | Self::Branch
                | Self::Check
                | Self::Switch
                | Self::Panic
                | Self::UnwindResume
                | Self::Abort
                | Self::Unreachable
        ) || self.is_invoke()
            || self.is_tail_call()
    }

    /// Return the MIR token type for one identifier.
    pub(crate) fn from_identifier(text: &str) -> Self {
        match text {
            "external" => Self::External,
            "export" => Self::Export,
            "function" => Self::Function,
            "global" => Self::Global,
            "shared" => Self::Shared,
            "constant" => Self::Constant,
            "type" => Self::Type,
            "block" => Self::Block,
            "local" => Self::Local,
            "return" => Self::Return,
            "jump" => Self::Jump,
            "branch" => Self::Branch,
            "check" => Self::Check,
            "switch" => Self::Switch,
            "panic" => Self::Panic,
            "unwind.resume" => Self::UnwindResume,
            "abort" => Self::Abort,
            "unreachable" => Self::Unreachable,
            "tail.call" => Self::TailCall,
            "call" => Self::Call,
            "call.indirect" => Self::CallIndirect,
            "tail.call.indirect" => Self::TailCallIndirect,
            "call.virtual" => Self::CallVirtual,
            "tail.call.virtual" => Self::TailCallVirtual,
            "call.dynamic" => Self::CallDynamic,
            "tail.call.dynamic" => Self::TailCallDynamic,
            "tail.call.witness" => Self::TailCallWitness,
            "invoke" => Self::Invoke,
            "invoke.indirect" => Self::InvokeIndirect,
            "invoke.virtual" => Self::InvokeVirtual,
            "invoke.dynamic" => Self::InvokeDynamic,
            "invoke.witness" => Self::InvokeWitness,
            "void" => Self::Void,
            "boolean" => Self::Boolean,
            "ref" => Self::Ref,
            "vector" => Self::Vector,
            "struct" => Self::Struct,
            "newtype" => Self::Newtype,
            "true" | "false" => Self::BooleanLiteral,
            "owned" | "borrowed" | "copy" => Self::Ownership,
            "readonly" => Self::Readonly,
            "const" => Self::Const,
            // null names a constant and a type, both read as an identifier
            "null" => Self::Identifier,
            _ if Type::from_primitive_name(text).is_some() => Self::TypeName,
            _ => Self::Identifier,
        }
    }
}
