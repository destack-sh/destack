use crate::{
    BindingModifier, Expression, FunctionSignature, Key, LocalNodeId, Node, NodeType, Parameter,
    Path, ScalarLiteral, StringId,
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
    /// Object type (any non-primitive).
    Object,
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
    /// `as comptime`
    AsComptime,
    /// `as const`
    AsConst,
}

impl TypeUnaryOperator {
    /// Whether the type unary operator is a prefix operator.
    pub fn is_prefix(&self) -> bool {
        matches!(
            self,
            Self::Not | Self::Type | Self::Readonly | Self::Typeof | Self::Keyof
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

/// One mapped type modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeModifier {
    /// Emit the plain modifier without an explicit sign.
    Present,
    /// Add the modifier with an explicit `+` sign.
    Add,
    /// Remove the modifier.
    Remove,
    /// Leave the modifier unspecified.
    None,
}

/// One mapped type modifier set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeMappedModifiers {
    /// The readonly modifier.
    pub readonly: TypeModifier,
    /// The optional modifier.
    pub optional: TypeModifier,
}

/// One mapped type parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeMappedParameter {
    /// The parameter name.
    pub name: StringId,
    /// The parameter constraint.
    pub constraint: LocalNodeId<Type>,
    /// The optional key remap.
    pub key_remap: Option<LocalNodeId<Type>>,
}

/// One type predicate subject.
#[derive(Debug, Clone, PartialEq)]
pub enum TypePredicateSubject {
    /// One named subject.
    Name(StringId),
    /// The `this` subject.
    This,
}

/// One type template literal.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeTemplateLiteral {
    /// The raw template strings.
    pub strings: Vec<StringId>,
    /// The interpolated type spans.
    pub spans: Vec<LocalNodeId<Type>>,
}

/// A Type is a Typescript type.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Scalar type literal.
    Scalar(TypeLiteral),
    /// This type.
    This,
    /// Path to something.
    Path {
        path: Path,
        static_arguments: Option<Vec<LocalNodeId<Type>>>,
    },
    /// Expression (unevaluated).
    Expression(LocalNodeId<Expression>),
    /// Conditional type.
    Conditional {
        left: LocalNodeId<Type>,
        right: LocalNodeId<Type>,
        then_type: LocalNodeId<Type>,
        else_type: LocalNodeId<Type>,
    },
    /// Mapped type.
    Mapped {
        parameter: TypeMappedParameter,
        modifiers: TypeMappedModifiers,
        value: LocalNodeId<Type>,
    },
    /// Index access type.
    Index {
        left: LocalNodeId<Type>,
        index: LocalNodeId<Type>,
    },
    /// Template literal type.
    TemplateLiteral(TypeTemplateLiteral),
    /// Import type.
    Import {
        target: StringId,
        qualifier: Option<Path>,
        static_arguments: Option<Vec<LocalNodeId<Type>>>,
    },
    /// Infer type binding.
    Infer {
        name: StringId,
        constraint: Option<LocalNodeId<Type>>,
    },
    /// Type predicate.
    Predicate {
        asserts: bool,
        subject: TypePredicateSubject,
        target: Option<LocalNodeId<Type>>,
    },

    /// Type unary operator.
    Unary {
        operator: TypeUnaryOperator,
        right: LocalNodeId<Type>,
    },
    /// Binary operator.
    Binary {
        left: LocalNodeId<Type>,
        operator: TypeBinaryOperator,
        right: LocalNodeId<Type>,
    },

    /// Array type `T[]`.
    Array { element: Option<LocalNodeId<Type>> },
    /// Tuple type `[T1, T2, ...]`.
    Tuple {
        elements: Vec<LocalNodeId<TupleElement>>,
    },
    /// Object type `{ a: T1, b: T2, ... }`.
    Object {
        properties: Vec<LocalNodeId<TypeField>>,
    },
    /// Union type `A | B | C`.
    Union { elements: Vec<LocalNodeId<Type>> },
    /// Intersection type `A & B & C`.
    Intersection { elements: Vec<LocalNodeId<Type>> },
    /// Function type `(T1, T2, ...) -> T`.
    Function { signature: FunctionSignature },

    /// Error type that could not be resolved.
    Error,
}

impl Node for Type {
    const TYPE: NodeType = NodeType::Type;
}

/// One tuple type element.
#[derive(Debug, Clone, PartialEq)]
pub struct TupleElement {
    /// The optional element label.
    pub label: Option<StringId>,
    /// The element type.
    pub ty: LocalNodeId<Type>,
    /// Whether the element is optional.
    pub is_optional: bool,
    /// Whether the element is readonly.
    pub is_readonly: bool,
    /// Whether the element is a rest element.
    pub is_rest: bool,
}

impl Node for TupleElement {
    const TYPE: NodeType = NodeType::TupleElement;
}

/// The type of an attribute (like a property or field).
#[derive(Debug, Clone, PartialEq)]
pub enum TypeField {
    /// Named field (like `a: T`).
    Field {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        ty: LocalNodeId<Type>,
    },
    /// Named method (like `foo(): T`).
    Method {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        signature: FunctionSignature,
    },
    /// Index signature (like `[key: string]: T`).
    IndexSignature {
        modifiers: Option<BindingModifier>,
        name: StringId,
        key_type: LocalNodeId<Type>,
        value_type: LocalNodeId<Type>,
    },
}

impl Node for TypeField {
    const TYPE: NodeType = NodeType::TypeField;
}

/// The polymorphism of some type or declaration.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Generics {
    /// The static parameters of the declaration.
    pub static_parameters: Option<Vec<LocalNodeId<Parameter>>> = None,
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
    pub extends_types: Option<Vec<LocalNodeId<Type>>> = None,
    /// The implements types of the declaration.
    pub implements_types: Option<Vec<LocalNodeId<Type>>> = None,
}

impl Heritage {
    /// Check whether the heritage lists any relations.
    pub fn is_empty(&self) -> bool {
        self.extends_types.is_none() && self.implements_types.is_none()
    }
}
