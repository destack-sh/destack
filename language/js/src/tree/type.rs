use serde::{Deserialize, Serialize};

use crate::{
    BindingModifier, FunctionSignature, GenericParameter, Key, LocalNodeId, Node, NodeType,
    Parameter, Path, ScalarLiteral, StringId,
};

/// A PrimitiveType is a primitive type node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// One mapped type modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MappedTypeModifier {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeMappedModifiers {
    /// The readonly modifier.
    pub readonly: MappedTypeModifier,
    /// The optional modifier.
    pub optional: MappedTypeModifier,
}

/// One mapped type parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeMappedParameter {
    /// The parameter name.
    pub name: StringId,
    /// The source type iterated by `in`.
    pub source_type: LocalNodeId<TypeExpression>,
    /// The optional key remap.
    pub key_remap: Option<LocalNodeId<TypeExpression>>,
}

/// One type predicate subject.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypePredicateSubject {
    /// One identifier subject.
    Identifier(StringId),
    /// The `this` subject.
    This,
}

/// One type template literal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeTemplateLiteral {
    /// The raw template strings.
    pub strings: Vec<StringId>,
    /// The interpolated type spans.
    pub spans: Vec<LocalNodeId<TypeExpression>>,
}

/// One function type declaration in type space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionTypeDeclaration {
    /// The generic parameters of the function type.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
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
    /// Whether the constructor type is abstract.
    pub is_abstract: bool,
    /// The generic parameters of the constructor type.
    pub generic_parameters: Vec<LocalNodeId<GenericParameter>>,
    /// The parameters of the constructor type.
    pub parameters: Vec<LocalNodeId<Parameter>>,
    /// The return type of the constructor type.
    pub return_type: Option<LocalNodeId<TypeExpression>>,
}

/// A TypeExpression is a TypeScript type expression.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeExpression {
    /// Scalar type literal.
    Scalar(TypeLiteral),
    /// This type.
    This,
    /// Path to something.
    Path {
        path: Path,
        generic_arguments: Vec<LocalNodeId<TypeExpression>>,
    },
    /// `readonly T`.
    Readonly {
        target_type: LocalNodeId<TypeExpression>,
    },
    /// `keyof T`.
    KeyOf {
        target_type: LocalNodeId<TypeExpression>,
    },
    /// `T!`.
    Must {
        target_type: LocalNodeId<TypeExpression>,
    },
    /// `!T`.
    Not {
        target_type: LocalNodeId<TypeExpression>,
    },
    /// `T extends U`.
    Extends {
        left: LocalNodeId<TypeExpression>,
        right: LocalNodeId<TypeExpression>,
    },
    /// `T implements U`.
    Implements {
        left: LocalNodeId<TypeExpression>,
        right: LocalNodeId<TypeExpression>,
    },
    /// Conditional type.
    Conditional {
        left: LocalNodeId<TypeExpression>,
        right: LocalNodeId<TypeExpression>,
        then_type: LocalNodeId<TypeExpression>,
        else_type: LocalNodeId<TypeExpression>,
    },
    /// Mapped type.
    Mapped {
        parameter: TypeMappedParameter,
        modifiers: TypeMappedModifiers,
        value: Option<LocalNodeId<TypeExpression>>,
    },
    /// Index access type.
    Index {
        left: LocalNodeId<TypeExpression>,
        index: LocalNodeId<TypeExpression>,
    },
    /// Template literal type.
    TemplateLiteral(TypeTemplateLiteral),
    /// Import type.
    Import {
        target: StringId,
        qualifier: Option<Path>,
        generic_arguments: Vec<LocalNodeId<TypeExpression>>,
    },
    /// Infer type binding.
    Infer {
        name: StringId,
        constraint: Option<LocalNodeId<TypeExpression>>,
    },
    /// Type predicate.
    Predicate {
        asserts: bool,
        subject: TypePredicateSubject,
        target: Option<LocalNodeId<TypeExpression>>,
    },

    /// Array type `T[]`.
    Array {
        element: LocalNodeId<TypeExpression>,
    },
    /// Tuple type `[T1, T2, ...]`.
    Tuple {
        elements: Vec<LocalNodeId<TupleElement>>,
    },
    /// Object type `{ a: T1, b: T2, ... }`.
    Object {
        members: Vec<LocalNodeId<TypeMember>>,
    },
    /// Union type `A | B | C`.
    Union {
        elements: Vec<LocalNodeId<TypeExpression>>,
    },
    /// Intersection type `A & B & C`.
    Intersection {
        elements: Vec<LocalNodeId<TypeExpression>>,
    },
    /// Function type declaration.
    FunctionTypeDeclaration(FunctionTypeDeclaration),
    /// Constructor type declaration.
    ConstructorTypeDeclaration(ConstructorTypeDeclaration),

    /// Error type that could not be resolved.
    Error,
}

impl Node for TypeExpression {
    const TYPE: NodeType = NodeType::TypeExpression;
}

/// One tuple type element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TupleElement {
    /// The optional element label.
    pub label: Option<StringId>,
    /// The element type.
    pub ty: LocalNodeId<TypeExpression>,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeMember {
    /// Named field (like `a: T`).
    Field {
        modifiers: Option<BindingModifier>,
        key: Key,
        ty: LocalNodeId<TypeExpression>,
    },
    /// Named method (like `foo(): T`).
    Method {
        modifiers: Option<BindingModifier>,
        key: Key,
        signature: FunctionSignature,
    },
    /// Call signature (like `<T>(value: T): U`).
    CallSignature {
        modifiers: Option<BindingModifier>,
        signature: FunctionTypeDeclaration,
    },
    /// Construct signature (like `new <T>(value: T): U`).
    ConstructSignature {
        modifiers: Option<BindingModifier>,
        signature: ConstructorTypeDeclaration,
    },
    /// Index signature (like `[key: string]: T`).
    IndexSignature {
        modifiers: Option<BindingModifier>,
        name: StringId,
        key_type: LocalNodeId<TypeExpression>,
        value_type: LocalNodeId<TypeExpression>,
    },
}

impl Node for TypeMember {
    const TYPE: NodeType = NodeType::TypeMember;
}
