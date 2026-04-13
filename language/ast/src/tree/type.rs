use serde::{Deserialize, Serialize};

use crate::{
    Argument, Declaration, Expression, FunctionSignature, GenericArgument, Key, LocalNodeId,
    Mutability, Node, NodeType, Path, ScalarLiteral, StringId, TypeLiteral, VarianceBound,
};

/// One object type property.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeProperty {
    /// Named field.
    Field {
        is_optional: bool,
        is_readonly: bool,
        key: Key,
        declared_type: LocalNodeId<TypeExpression>,
    },
    /// Named method.
    Method {
        is_optional: bool,
        key: Option<Key>,
        signature: FunctionSignature,
    },
    /// Index signature.
    IndexSignature {
        is_readonly: bool,
        name: StringId,
        key_type: LocalNodeId<TypeExpression>,
        value_type: LocalNodeId<TypeExpression>,
    },
    /// Malformed type property slot.
    Error,
}

impl Node for TypeProperty {
    const TYPE: NodeType = NodeType::TypeProperty;
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

    /// Tuple type syntax.
    Tuple {
        elements: Vec<LocalNodeId<GenericArgument>>,
    },

    /// Array type syntax.
    Array {
        element: LocalNodeId<TypeExpression>,
    },

    /// Object type syntax.
    Object {
        properties: Vec<LocalNodeId<TypeProperty>>,
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
