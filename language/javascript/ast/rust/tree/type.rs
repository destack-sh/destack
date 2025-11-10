use crate::{Definition, Node, NodeId, NodeType, ScalarLiteral};

/// A PrimitiveType is a primitive type node.
#[derive(Debug, Clone, PartialEq)]
pub enum PrimitiveType {
    /// Boolean type.
    Boolean,
    /// String type (unsized).
    String,
    /// Bigint type (unsized).
    Bigint,
    /// "Number" type (alias).
    Number,
    /// Symbol type.
    Symbol,
    /// Unique symbol type.
    UniqueSymbol,
}

/// A TypeLiteral is a scalar type.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeLiteral {
    /// Never type `never`.
    Never,
    /// Any type `any`.
    Any,
    /// Undefined type and value.
    Undefined,
    /// Unknown type.
    Unknown,
    /// Void type.
    Void,
    /// Null type and value.
    Null,
    /// Primitive type.
    Primitive(PrimitiveType),
    /// Scalar literal.
    ScalarLiteral(ScalarLiteral),
}

/// A TypeUnaryOperator is a type unary operator.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TypeUnaryOperator {
    /// Not `!T`.
    Not,
    /// Maybe 'T?'.
    Maybe,
    /// Must 'T!'.
    Must,
    /// `type`
    Type,
    /// `readonly`
    Readonly,
    /// `typeof`
    Typeof,
    /// `keyof`
    Keyof,
    /// `infer`
    Infer,
    /// `as const`
    AsConst,
    /// `asserts`
    Asserts,
}

/// A TypeBinaryOperator is a type binary operator.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TypeBinaryOperator {
    /// `as`
    Cast,
    /// `in`
    In,
    /// `is`
    Is,
    /// `instanceof`
    InstanceOf,
    /// `satisfies`
    Satisfies,
    /// `extends`
    Extends,
    /// `implements`
    Implements,
}

/// A Type is a Typescript type.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Scalar type literal.
    Scalar(TypeLiteral),
    /// Evaluated definition type.
    Definition(NodeId<Definition>),

    /// Type unary operator.
    Unary {
        operator: TypeUnaryOperator,
        right: NodeId<Type>,
    },
    /// Binary
    Binary {
        left: NodeId<Type>,
        operator: TypeBinaryOperator,
        right: NodeId<Type>,
    },
}

impl Node for Type {
    const TYPE: NodeType = NodeType::Type;
}
