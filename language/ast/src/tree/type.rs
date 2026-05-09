use serde::{Deserialize, Serialize};

use crate::{
    Declaration, Expression, FunctionSignature, GenericArgument, GenericParameter, Key,
    LocalNodeId, Mutability, Node, NodeType, Parameter, Path, ScalarLiteral, StringId,
    TupleElement, TypeLiteral, VarianceBound, WhereClause,
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
    CallSignature { signature: FunctionTypeDeclaration },
    /// Construct signature declaration.
    ConstructSignature {
        signature: ConstructorTypeDeclaration,
    },
    /// Index signature.
    IndexSignature {
        name: StringId,
        key_type: LocalNodeId<TypeExpression>,
        value_type: LocalNodeId<TypeExpression>,
        is_optional: bool,
        is_readonly: bool,
    },
    /// Type embedding.
    Embed { value: LocalNodeId<TypeExpression> },
    /// Associated type requirement or definition.
    AssociatedType {
        name: StringId,
        generic_parameters: Vec<LocalNodeId<GenericParameter>>,
        where_clauses: Vec<LocalNodeId<WhereClause>>,
        constraint: Option<LocalNodeId<TypeExpression>>,
        value: Option<LocalNodeId<TypeExpression>>,
    },
    /// Associated compile-time constant requirement or definition.
    AssociatedConst {
        name: StringId,
        declared_type: Option<LocalNodeId<TypeExpression>>,
        value: Option<LocalNodeId<Expression>>,
    },
    /// Malformed type member slot.
    Error,
}

impl Node for TypeMember {
    const TYPE: NodeType = NodeType::TypeMember;
}

impl TypeMember {
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

/// A type predicate subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypePredicateSubject {
    /// Identifier subject.
    Identifier(StringId),
    /// `this` subject.
    This,
}

/// One function type declaration in type space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionTypeDeclaration {
    /// The generic parameters of the function type.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The where clauses of the function type.
    pub where_clauses: Vec<LocalNodeId<WhereClause>>,
    /// The optional `this` parameter.
    pub this_parameter: Option<LocalNodeId<Parameter>>,
    /// The parameters of the function type.
    pub parameters: Vec<LocalNodeId<Parameter>>,
    /// The return type of the function type.
    pub return_type: Option<LocalNodeId<TypeExpression>>,
}

/// One constructor type declaration in type space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConstructorTypeDeclaration {
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
        length: LocalNodeId<TypeExpression>,
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

    /// Embedded declaration type.
    ///
    /// Examples:
    /// ```
    /// struct User { name: string }
    /// interface Named { name: string }
    /// ```
    Declaration {
        declaration: LocalNodeId<Declaration>,
    },

    /// Function type declaration.
    ///
    /// Examples:
    /// ```
    /// (value: T) => U
    /// <T>(value: T): T
    /// ```
    FunctionTypeDeclaration(FunctionTypeDeclaration),

    /// Constructor type declaration.
    ///
    /// Examples:
    /// ```
    /// new (value: string) => User
    /// abstract new <T>(value: T): Box<T>
    /// ```
    ConstructorTypeDeclaration(ConstructorTypeDeclaration),

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
    TypeOfValue { value: LocalNodeId<Expression> },

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

    /// `T as comptime`.
    ///
    /// Examples:
    /// ```
    /// T as comptime
    /// typeof value as comptime
    /// ```
    AsComptime {
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

    /// Mapped type.
    ///
    /// Examples:
    /// ```
    /// { [K in keyof T]: T[K] }
    /// { readonly [K in keyof T]?: T[K] }
    /// ```
    Mapped {
        parameter: TypeMappedParameter,
        readonly: MappedTypeModifier,
        optional: MappedTypeModifier,
        value: LocalNodeId<TypeExpression>,
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

    /// Infer binding.
    ///
    /// Examples:
    /// ```
    /// infer T
    /// infer Item extends string
    /// ```
    Infer {
        name: StringId,
        constraint: Option<LocalNodeId<TypeExpression>>,
    },

    /// Type predicate.
    ///
    /// Examples:
    /// ```
    /// value is Foo
    /// asserts value is Foo
    /// asserts this is Ready
    /// ```
    Predicate {
        asserts: bool,
        subject: TypePredicateSubject,
        target: Option<LocalNodeId<TypeExpression>>,
    },

    /// Missing type child.
    Missing,

    /// Error placeholder.
    Error,
}

impl Node for TypeExpression {
    const TYPE: NodeType = NodeType::TypeExpression;
}
