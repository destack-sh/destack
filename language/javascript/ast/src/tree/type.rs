use crate::{
    Argument, BindingModifier, Expression, FunctionSignature, Key, Node, NodeId, NodeType,
    Parameter, Path, ScalarLiteral,
};

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

impl TypeUnaryOperator {
    /// Whether the type unary operator is a prefix operator.
    pub fn is_prefix(&self) -> bool {
        matches!(
            self,
            Self::Not
                | Self::Type
                | Self::Readonly
                | Self::Typeof
                | Self::Keyof
                | Self::Infer
                | Self::AsConst
                | Self::Asserts
        )
    }
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
    /// Path to something.
    Path {
        path: Path,
        static_arguments: Option<Vec<NodeId<Argument>>>,
    },
    /// Expression (unevaluated).
    Expression(NodeId<Expression>),

    /// Type unary operator.
    Unary {
        operator: TypeUnaryOperator,
        right: NodeId<Type>,
    },
    /// Binary operator.
    Binary {
        left: NodeId<Type>,
        operator: TypeBinaryOperator,
        right: NodeId<Type>,
    },

    /// Array type `T[]`.
    Array { element: NodeId<Type> },
    /// Tuple type `[T1, T2, ...]`.
    Tuple { elements: Vec<NodeId<Type>> },
    /// Object type `{ a: T1, b: T2, ... }`.
    Object { properties: Vec<NodeId<TypeField>> },
    /// Union type `A | B | C`.
    Union { elements: Vec<NodeId<Type>> },
    /// Intersection type `A & B & C`.
    Intersection { elements: Vec<NodeId<Type>> },
    /// Function type `(T1, T2, ...) -> T`.
    Function { signature: FunctionSignature },

    /// Error type that could not be resolved.
    Error,
}

impl Node for Type {
    const TYPE: NodeType = NodeType::Type;
}

/// The type of an attribute (like a property or field).
#[derive(Debug, Clone, PartialEq)]
pub enum TypeField {
    /// Named field (like `a: T`).
    Field {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        ty: NodeId<Type>,
    },
    /// Named method (like `foo(): T`).
    Method {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        signature: FunctionSignature,
    },
}

impl Node for TypeField {
    const TYPE: NodeType = NodeType::TypeField;
}

/// The polymorphism of some type or declaration.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Generics {
    /// The static parameters of the declaration.
    pub static_parameters: Option<Vec<NodeId<Parameter>>> = None,
}

impl Generics {
    /// Check whether there are any generic parameters.
    pub fn is_empty(&self) -> bool {
        self.static_parameters.is_none()
    }
}

/// The polymoprhic relations.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Heritage {
    /// The extends types of the declaration.
    pub extends_types: Option<Vec<NodeId<Type>>> = None,
    /// The implements types of the declaration.
    pub implements_types: Option<Vec<NodeId<Type>>> = None,
}

impl Heritage {
    /// Check whether the heritage lists any relations.
    pub fn is_empty(&self) -> bool {
        self.extends_types.is_none() && self.implements_types.is_none()
    }
}
