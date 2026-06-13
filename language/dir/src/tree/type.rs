use serde::{Deserialize, Serialize};

use crate::{
    Expression, FunctionSignature, GenericArgument, GenericParameter, Key, LocalNodeId, Mutability,
    Node, NodeType, Parameter, Path, RangeEnd, ScalarLiteral, ScopeKind, StaticKey, StringId,
    SymbolKind, SymbolSpace, ThisForm, TupleElement, TypeLiteral, VarianceBound, View, WhereClause,
};

/// One type-surface member.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeMember {
    /// Named field.
    Field {
        key: Key,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        is_static: bool,
        is_optional: bool,
        is_readonly: bool,
    },
    /// Named method.
    Method {
        key: Key,
        signature: FunctionSignature,
        body: Option<LocalNodeId<Expression>>,
        is_static: bool,
        is_optional: bool,
    },
    /// Call signature declaration.
    CallSignature {
        /// The call signature function type expression.
        signature: FunctionTypeExpression,
    },
    /// Construct signature declaration.
    ConstructSignature { signature: ConstructorType },
    /// Index signature.
    IndexSignature {
        name: StringId,
        key_type: LocalNodeId<TypeExpression>,
        value_type: LocalNodeId<TypeExpression>,
        is_optional: bool,
        is_readonly: bool,
    },
    /// Associated type requirement or definition.
    AssociatedType {
        name: StringId,
        generic_parameters: Vec<LocalNodeId<GenericParameter>>,
        where_clauses: Vec<LocalNodeId<WhereClause>>,
        constraint: Option<LocalNodeId<TypeExpression>>,
        value: Option<LocalNodeId<TypeExpression>>,
        is_abstract: bool,
        is_override: bool,
    },
    /// Associated compile-time constant requirement or definition.
    AssociatedConst {
        name: StringId,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        value: Option<LocalNodeId<Expression>>,
        is_abstract: bool,
        is_override: bool,
    },
    /// Malformed type member slot.
    Error,
}

impl Node for TypeMember {
    const TYPE: NodeType = NodeType::TypeMember;
}

impl TypeMember {
    /// Return the symbol key introduced by this type member.
    pub fn symbol_key(&self) -> Option<StaticKey> {
        match self {
            Self::AssociatedType { name, .. } | Self::AssociatedConst { name, .. } => {
                Some(StaticKey::Name(*name))
            }
            Self::Field { key, .. } => key.direct_static_key(),
            Self::Method { key, .. } => key.direct_static_key(),
            Self::CallSignature { .. }
            | Self::ConstructSignature { .. }
            | Self::IndexSignature { .. }
            | Self::Error => None,
        }
    }

    /// Return the symbol kind introduced by this type member.
    pub fn symbol_kind(&self) -> Option<SymbolKind> {
        match self {
            Self::AssociatedType { .. } => Some(SymbolKind::AssociatedType),
            Self::AssociatedConst { .. } => Some(SymbolKind::AssociatedConst),
            Self::Field { .. } => Some(SymbolKind::Variable),
            Self::Method { .. } => Some(SymbolKind::Function),
            Self::CallSignature { .. }
            | Self::ConstructSignature { .. }
            | Self::IndexSignature { .. }
            | Self::Error => None,
        }
    }

    /// Return the symbol space introduced by this type member.
    pub fn symbol_space(&self) -> Option<SymbolSpace> {
        self.symbol_kind().map(SymbolKind::symbol_space)
    }

    /// Return the owned scope kind for this type member symbol.
    pub fn symbol_scope_kind(&self) -> Option<ScopeKind> {
        match self {
            Self::AssociatedType { .. } => Some(ScopeKind::Type),
            Self::Method { .. } => Some(ScopeKind::Function),
            _ => None,
        }
    }

    /// Get the declared name of the type member when one exists.
    pub fn name(&self) -> Option<StringId> {
        match self {
            TypeMember::AssociatedType { name, .. } | TypeMember::AssociatedConst { name, .. } => {
                Some(*name)
            }
            _ => None,
        }
    }

    /// Get the key of the type member when one exists.
    pub fn key(&self) -> Option<&Key> {
        match self {
            TypeMember::Field { key, .. } => Some(key),
            TypeMember::Method { key, .. } => Some(key),
            _ => None,
        }
    }

    /// Get the function signature of the type member when one exists.
    pub fn signature(&self) -> Option<&FunctionSignature> {
        match self {
            TypeMember::Method { signature, .. } => Some(signature),
            _ => None,
        }
    }

    /// Return whether this type member has a `static` modifier.
    pub const fn has_static_modifier(&self) -> bool {
        match self {
            TypeMember::Field { is_static, .. } | TypeMember::Method { is_static, .. } => {
                *is_static
            }
            TypeMember::CallSignature { .. }
            | TypeMember::ConstructSignature { .. }
            | TypeMember::IndexSignature { .. }
            | TypeMember::AssociatedType { .. }
            | TypeMember::AssociatedConst { .. }
            | TypeMember::Error => false,
        }
    }

    /// Return whether this type member body has an implicit receiver.
    pub const fn has_implicit_receiver(&self) -> bool {
        match self {
            TypeMember::Field { is_static, .. } | TypeMember::Method { is_static, .. } => {
                !*is_static
            }
            TypeMember::CallSignature { .. }
            | TypeMember::ConstructSignature { .. }
            | TypeMember::IndexSignature { .. }
            | TypeMember::AssociatedType { .. }
            | TypeMember::AssociatedConst { .. } => true,
            TypeMember::Error => false,
        }
    }

    /// Return whether this type member belongs to the instance surface.
    pub const fn is_instance_member(&self) -> bool {
        match self {
            TypeMember::Field { is_static, .. } | TypeMember::Method { is_static, .. } => {
                !*is_static
            }
            TypeMember::IndexSignature { .. } => true,
            TypeMember::CallSignature { .. }
            | TypeMember::ConstructSignature { .. }
            | TypeMember::AssociatedType { .. }
            | TypeMember::AssociatedConst { .. }
            | TypeMember::Error => false,
        }
    }
}

/// A mapped type parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeMappedParameter {
    /// The parameter name.
    pub name: StringId,
    /// The source type iterated by `in`.
    pub source_type: LocalNodeId<TypeExpression>,
    /// The optional key remap.
    pub key_remap: Option<LocalNodeId<TypeExpression>>,
}

impl Node for TypeMappedParameter {
    const TYPE: NodeType = NodeType::TypeMappedParameter;
}

/// A mapped-type modifier sign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MappedTypeModifier {
    /// The plain modifier without an explicit sign.
    Present,
    /// Add a modifier with an explicit `+` sign.
    Add,
    /// Remove a modifier with an explicit `-` sign.
    Remove,
    /// No modifier specified.
    None,
}

/// One function type expression in type space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionTypeExpression {
    /// The generic parameters of the function type.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the function type.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The source form used for the receiver.
    pub this_form: Option<ThisForm>,
    /// The optional `this` parameter.
    pub this_parameter: Option<LocalNodeId<Parameter>>,
    /// The parameters of the function type.
    pub parameters: Vec<LocalNodeId<Parameter>>,
    /// The return type of the function type.
    pub return_type: Option<LocalNodeId<TypeExpression>>,
}

impl FunctionTypeExpression {
    /// Return whether this function type declares a generic template.
    pub fn declares_generic_template(&self, view: &View<'_>) -> bool {
        // explicit generic header
        if !self.generic_parameters.is_empty() {
            return true;
        }

        // comptime receiver parameter
        if let Some(parameter) = self.this_parameter
            && view.get(parameter).is_comptime()
        {
            return true;
        }

        // comptime parameters
        self.parameters
            .iter()
            .any(|parameter| view.get(*parameter).is_comptime())
    }
}

/// One constructor type in type space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstructorType {
    /// The generic parameters of the constructor type.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the constructor type.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The parameters of the constructor type.
    pub parameters: Vec<LocalNodeId<Parameter>>,
    /// The return type of the constructor type.
    pub return_type: Option<LocalNodeId<TypeExpression>>,
    /// Whether the constructor type is abstract.
    pub is_abstract: bool,
}

impl ConstructorType {
    /// Return whether this constructor type declares a generic template.
    pub fn declares_generic_template(&self, view: &View<'_>) -> bool {
        // explicit generic header
        if !self.generic_parameters.is_empty() {
            return true;
        }

        // comptime parameters
        self.parameters
            .iter()
            .any(|parameter| view.get(*parameter).is_comptime())
    }
}

/// The parsed form of an infer type expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InferForm {
    /// Anonymous `_` type inference hole.
    Hole,
    /// Explicit `infer T` or `infer _` binding expression.
    Infer,
}

/// A type-space expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeExpression {
    /// Parenthesized type expression.
    ///
    /// Examples:
    /// ```
    /// (T)
    /// (string | number)
    /// ```
    Parenthesized {
        expression: LocalNodeId<TypeExpression>,
    },

    /// Scalar literal type.
    ///
    /// Examples:
    /// ```
    /// "ok"
    /// 42
    /// true
    /// ```
    ScalarLiteral { value: ScalarLiteral },

    /// Literal type.
    ///
    /// Examples:
    /// ```
    /// null
    /// undefined
    /// ```
    Literal { value: TypeLiteral },

    /// Bare `intrinsic` marker in type space.
    ///
    /// Examples:
    /// ```
    /// intrinsic
    /// ```
    Intrinsic,

    /// Parenthesized tuple type.
    ///
    /// Examples:
    /// ```
    /// (A, B)
    /// (name: string, age: number)
    /// ```
    Tuple {
        elements: Vec<LocalNodeId<TupleElement>>,
    },

    /// Bracket tuple type.
    ///
    /// Examples:
    /// ```
    /// [A, B]
    /// [name: string, age: number]
    /// ```
    ArrayTuple {
        elements: Vec<LocalNodeId<TupleElement>>,
    },

    /// Homogeneous array type.
    ///
    /// Examples:
    /// ```
    /// T[]
    /// string[]
    /// ```
    Array {
        element: LocalNodeId<TypeExpression>,
    },

    /// Runtime-length homogeneous view type.
    ///
    /// Examples:
    /// ```
    /// [T]
    /// [byte]
    /// ```
    Slice {
        /// The element type.
        element: LocalNodeId<TypeExpression>,
    },

    /// Fixed-length array type.
    ///
    /// Examples:
    /// ```
    /// [T; N]
    /// [byte; 32]
    /// ```
    FixedArray {
        /// The element type.
        element: LocalNodeId<TypeExpression>,
        /// The length expression.
        length: LocalNodeId<Expression>,
    },

    /// Object type.
    ///
    /// Examples:
    /// ```
    /// { name: string }
    /// { readonly id: string; age?: number }
    /// ```
    Object {
        members: Vec<LocalNodeId<TypeMember>>,
    },

    /// Function type.
    ///
    /// Examples:
    /// ```
    /// (value: T) => U
    /// <T>(value: T): T
    /// ```
    Function(FunctionTypeExpression),

    /// Constructor type.
    ///
    /// Examples:
    /// ```
    /// new (value: string) => User
    /// abstract new <T>(value: T): Box<T>
    /// ```
    Constructor(ConstructorType),

    /// Qualified type reference with optional generic arguments.
    ///
    /// Examples:
    /// ```
    /// Foo
    /// geom.Mesh<Point>
    /// Result<T, E>
    /// ```
    Reference {
        path: Path,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
    },

    /// Type member projection with optional generic arguments.
    ///
    /// Examples:
    /// ```
    /// T.Item
    /// Result<T, E>.Ok
    /// ```
    Member {
        left: LocalNodeId<TypeExpression>,
        name: StringId,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
    },

    /// Range type expression.
    ///
    /// Examples:
    /// ```
    /// 0..10
    /// 0..=10
    /// 0..
    /// ..10
    /// ..=10
    /// ..
    /// ```
    Range {
        start: Option<LocalNodeId<TypeExpression>>,
        end: Option<LocalNodeId<TypeExpression>>,
        end_kind: RangeEnd,
    },

    /// `const` in type space.
    ///
    /// Examples:
    /// ```
    /// const
    /// ```
    Const,

    /// `this` in type space.
    ///
    /// Examples:
    /// ```
    /// this
    /// ```
    This,

    /// `readonly T`.
    ///
    /// Examples:
    /// ```
    /// readonly string[]
    /// readonly [T]
    /// ```
    Readonly {
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `local T`.
    ///
    /// Examples:
    /// ```
    /// local User
    /// local ^User
    /// ```
    Local {
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `shared T`.
    ///
    /// Examples:
    /// ```
    /// shared User
    /// shared ^User
    /// ```
    Shared {
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `keyof T`.
    ///
    /// Examples:
    /// ```
    /// keyof T
    /// keyof User
    /// ```
    KeyOf {
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `typeof value`.
    ///
    /// Examples:
    /// ```
    /// typeof value
    /// typeof namespace.Member
    /// ```
    TypeOf { value: LocalNodeId<Expression> },

    /// `T!`.
    ///
    /// Examples:
    /// ```
    /// T!
    /// string!
    /// ```
    Must {
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `!T`.
    ///
    /// Examples:
    /// ```
    /// !T
    /// !false
    /// ```
    Not {
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `^T`.
    ///
    /// Examples:
    /// ```
    /// ^T
    /// ^mut T
    /// ```
    OwnedOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `&T`.
    ///
    /// Examples:
    /// ```
    /// &T
    /// &mut T
    /// ```
    BorrowedOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `*T`.
    ///
    /// Examples:
    /// ```
    /// *T
    /// *mut T
    /// ```
    PointerOf {
        mutability: Option<Mutability>,
        target_type: LocalNodeId<TypeExpression>,
    },

    /// Union type.
    ///
    /// Examples:
    /// ```
    /// string | number
    /// "ok" | "err"
    /// ```
    Union {
        elements: Vec<LocalNodeId<TypeExpression>>,
    },

    /// Intersection type.
    ///
    /// Examples:
    /// ```
    /// A & B
    /// Serializable & Display
    /// ```
    Intersection {
        elements: Vec<LocalNodeId<TypeExpression>>,
    },

    /// Conditional type.
    ///
    /// Examples:
    /// ```
    /// T extends U ? X : Y
    /// ```
    Conditional {
        left: LocalNodeId<TypeExpression>,
        extends_type: LocalNodeId<TypeExpression>,
        then_type: LocalNodeId<TypeExpression>,
        else_type: LocalNodeId<TypeExpression>,
    },

    /// Assignability relation.
    ///
    /// Examples:
    /// ```
    /// T extends U
    /// ```
    Extends {
        left: LocalNodeId<TypeExpression>,
        right: LocalNodeId<TypeExpression>,
    },

    /// Explicit conformance relation.
    ///
    /// Examples:
    /// ```
    /// T implements U
    /// ```
    Implements {
        left: LocalNodeId<TypeExpression>,
        right: LocalNodeId<TypeExpression>,
    },

    /// Mapped type.
    ///
    /// Examples:
    /// ```
    /// { [K in keyof T]: T[K] }
    /// { readonly [K in keyof T]?: T[K] }
    /// ```
    Mapped {
        parameter: LocalNodeId<TypeMappedParameter>,
        readonly: MappedTypeModifier,
        optional: MappedTypeModifier,
        value: Option<LocalNodeId<TypeExpression>>,
    },

    /// Indexed access type.
    ///
    /// Examples:
    /// ```
    /// T[K]
    /// Foo["bar"]
    /// ```
    Index {
        left: LocalNodeId<TypeExpression>,
        index: LocalNodeId<TypeExpression>,
    },

    /// Template literal type.
    ///
    /// Examples:
    /// ```
    /// `get${Name}`
    /// `${Prefix}_${Suffix}`
    /// ```
    TemplateLiteral {
        strings: Vec<StringId>,
        spans: Vec<LocalNodeId<TypeExpression>>,
    },

    /// An Infer expression is a named infer binding or anonymous inference hole.
    ///
    /// Examples:
    /// ```
    /// _
    /// infer T
    /// infer _
    /// infer Item extends string
    /// ```
    Infer {
        form: InferForm,
        name: Option<StringId>,
        constraint: Option<LocalNodeId<TypeExpression>>,
    },

    /// Missing type child.
    Missing,

    /// Error placeholder.
    Error,
}

impl Node for TypeExpression {
    const TYPE: NodeType = NodeType::TypeExpression;
}
