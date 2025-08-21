use std::collections::HashMap;

///
/// Generic
///

/// A position range in a `SourceFile`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

/// The identifier type used by AST nodes.
pub type NodeId = u32;

/// A file inside the `SourceMap`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub id: NodeId,
    pub path: String,
}

/// Keep track of files participating in the AST.
#[derive(Debug, Default, Clone)]
pub struct SourceMap {
    pub source_files: HashMap<NodeId, SourceFile>,
}

/// Item visibility similar to Rust with Destack-specific defaults.
#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Inherited,
    Pub,
    PubCrate,
    PubSuper,
    PubRestricted(Path),
}

/// A module path, possibly absolute, with optional generic arguments per segment.
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    pub is_absolute: bool,
    pub segments: Vec<PathSegment>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathSegment {
    pub ident: String,
    pub generic_args: Vec<GenericArg>,
}

//
// Definitions
//

#[derive(Debug, Clone, PartialEq)]
pub enum GenericArg {
    Type(Type),
    Const(ConstArg),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstArg {
    /// Constant value expressed as an expression evaluated at compile time.
    Expr,
}

/// Top-level items found in modules.
#[derive(Debug, Clone, PartialEq)]
pub enum ItemDefinition {
    Module(ModuleDefinition),
    Use(UseDeclaration),
    Struct(StructDefinition),
    Enum(EnumDefinition),
    Function(FunctionDefinition),
    Impl(ImplDefinition),
    Const(ConstItemDefinition),
    TypeAlias(TypeAliasDefinition),
}

/// A module definition containing a sequence of items.
#[derive(Debug, Clone, PartialEq)]
pub struct ModuleDefinition {
    pub id: NodeId,
    pub span: Span,
    pub visibility: Visibility,
    pub name: String,
    pub items: Vec<ItemDefinition>,
}

/// A `use` tree.
#[derive(Debug, Clone, PartialEq)]
pub struct UseDeclaration {
    pub id: NodeId,
    pub span: Span,
    pub visibility: Visibility,
    pub path: Path,
    pub alias: Option<String>,
    pub is_glob: bool,
}

/// A struct definition with named fields.
#[derive(Debug, Clone, PartialEq)]
pub struct StructDefinition {
    pub id: NodeId,
    pub span: Span,
    pub visibility: Visibility,
    pub name: String,
    pub generics: Generics,
    pub fields: Vec<StructFieldDefinition>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructFieldDefinition {
    pub span: Span,
    pub visibility: Visibility,
    pub name: String,
    pub ty: Type,
    pub default_value: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDefinition {
    pub id: NodeId,
    pub span: Span,
    pub visibility: Visibility,
    pub name: String,
    pub generics: Generics,
    pub repr: Option<Type>,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    pub span: Span,
    pub name: String,
    pub kind: EnumVariantKind,
    pub discriminant: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariantKind {
    Unit,
    Tuple(Vec<Type>),
    Struct(Vec<StructFieldDefinition>),
}

/// A function or method.
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDefinition {
    pub id: NodeId,
    pub span: Span,
    pub visibility: Visibility,
    pub name: String,
    pub generics: Generics,
    pub params: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: Option<BlockDefinition>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub span: Span,
    pub name: String,
    pub ty: Type,
}

/// An inherent or trait implementation block.
#[derive(Debug, Clone, PartialEq)]
pub struct ImplDefinition {
    pub id: NodeId,
    pub span: Span,
    pub generics: Generics,
    pub trait_path: Option<Path>,
    pub target_type: Type,
    pub items: Vec<ItemDefinition>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstItemDefinition {
    pub id: NodeId,
    pub span: Span,
    pub visibility: Visibility,
    pub name: String,
    pub ty: Type,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAliasDefinition {
    pub id: NodeId,
    pub span: Span,
    pub visibility: Visibility,
    pub name: String,
    pub generics: Generics,
    pub target: Type,
}

/// Generic parameters for items and impls.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Generics {
    pub type_params: Vec<TypeParam>,
    pub const_params: Vec<ConstParam>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParam {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstParam {
    pub name: String,
    pub ty: Type,
}

/// Types with support for nested composite constructions.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// A named type path like `Vector2`, `Option<Entity>` or `EntityReference<Entity>`.
    Path(Path),
    /// Tuple types like `(f32, f32)`.
    Tuple(Vec<Type>),
    /// A fixed-size array like `[T; N]`. `N` may be absent for slice-like forms.
    Array {
        element: Box<Type>,
        size: Option<Box<Expression>>,
    },
    /// A dynamically sized view `T[]` used in Destack sources.
    Slice(Box<Type>),
    /// Reference type `&T` or `&mut T`.
    Reference { is_mut: bool, inner: Box<Type> },
}

/// A block expression consisting of a sequence of statements.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockDefinition {
    pub span: Span,
    pub stmts: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Item(ItemDefinition),
    Let {
        pattern: Pattern,
        type_annotation: Option<Type>,
        value: Option<Expression>,
    },
    Expr(Expression),
    Semi(Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Wild,
    Ident(String),
    Tuple(Vec<Pattern>),
    Struct {
        path: Path,
        fields: Vec<(String, Option<Pattern>)>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Literal(Literal),
    Path(Path),
    Tuple(Vec<Expression>),
    Array(Vec<Expression>),
    StructLiteral {
        path: Path,
        fields: Vec<(String, Expression)>,
    },
    Field {
        base: Box<Expression>,
        member: String,
    },
    Index {
        base: Box<Expression>,
        index: Box<Expression>,
    },
    StaticCall {
        callee: Box<Expression>,
        args: Vec<Expression>,
    },
    DynamicCall {
        receiver: Box<Expression>,
        method: String,
        args: Vec<Expression>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expression>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Assign {
        target: Box<Expression>,
        value: Box<Expression>,
    },
    Block(BlockDefinition),
    If {
        cond: Box<Expression>,
        then_branch: BlockDefinition,
        else_branch: Option<Box<Expression>>, // `Block` or another `If` as expression
    },
    Return(Option<Box<Expression>>),
    Cast {
        expr: Box<Expression>,
        ty: Type,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Bool(bool),
    Integer(i128),
    Float(f64),
    String(String),
    Char(char),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}
