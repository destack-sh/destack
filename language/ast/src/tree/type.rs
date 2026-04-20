use serde::{Deserialize, Serialize};

use crate::{
    Argument, Declaration, Expression, FunctionSignature, GenericArgument, GenericParameter, Key,
    LocalNodeId, Mutability, Node, NodeType, Path, ScalarLiteral, StringId, TupleElement,
    TypeLiteral, VarianceBound, WhereClause,
};

/// One type-surface member.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeMember {
    /// Named field.
    Field {
        is_optional: bool,
        is_readonly: bool,
        key: Key,
        declared_type: Option<LocalNodeId<TypeExpression>>,
    },
    /// Named method.
    Method {
        is_optional: bool,
        key: Option<Key>,
        signature: FunctionSignature,
        body: Option<LocalNodeId<Expression>>,
    },
    /// Index signature.
    IndexSignature {
        is_optional: bool,
        is_readonly: bool,
        name: StringId,
        key_type: LocalNodeId<TypeExpression>,
        value_type: LocalNodeId<TypeExpression>,
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
            TypeMember::Method { key, .. } => key.as_ref(),
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
pub enum TypeModifier {
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

/// A type-space syntax node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeExpression {
    /// Parenthesized type expression.
    Parenthesized {
        expression: LocalNodeId<TypeExpression>,
    },

    /// Scalar literal type syntax.
    ScalarLiteral { value: ScalarLiteral },

    /// Literal type syntax.
    Literal { value: TypeLiteral },

    /// Bare `intrinsic` marker syntax in type space.
    Intrinsic,

    /// Tuple type syntax.
    Tuple {
        elements: Vec<LocalNodeId<TupleElement>>,
    },

    /// Array type syntax.
    Array {
        element: LocalNodeId<TypeExpression>,
    },

    /// Object type syntax.
    Object {
        members: Vec<LocalNodeId<TypeMember>>,
    },

    /// Embedded declaration type syntax.
    Declaration {
        declaration: LocalNodeId<Declaration>,
    },

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
    Const,

    /// `this` in type space.
    This,

    /// Type import expression.
    ///
    /// Examples:
    /// ```
    /// import("foo")
    /// import("foo").Bar
    /// import("foo", { with: { type: "json" } })
    /// ```
    Import {
        target: LocalNodeId<Expression>,
        arguments: Vec<LocalNodeId<Argument>>,
        qualifier: Option<Path>,
        generic_arguments: Vec<LocalNodeId<GenericArgument>>,
    },

    /// `readonly T`.
    Readonly {
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `keyof T`.
    KeyOf {
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `typeof value`.
    TypeOfValue { value: LocalNodeId<Expression> },

    /// `T!`.
    Must {
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `T as comptime`.
    AsComptime {
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `!T`.
    Not {
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `^T`.
    ValueOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `&T`.
    ReferenceOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        target_type: LocalNodeId<TypeExpression>,
    },

    /// `*T`.
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
        readonly: TypeModifier,
        optional: TypeModifier,
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
