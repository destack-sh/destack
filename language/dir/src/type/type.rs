use destack_source::{AdaptImage, ModuleId};
use serde::{Deserialize, Serialize};

use crate::{
    Asynchrony, FunctionCardinality, GlobalSymbolId, LocalNodeId, Mutability, Path, ScalarLiteral,
    StaticArgument, StaticKey, StringId, TypeExpression, VarianceBound,
};

use super::{PrimitiveType, TypeBinaryOperator, TypeUnaryOperator};

/// A TypeLiteral is a scalar type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum TypeLiteral {
    /// Never type `never`.
    Never,
    /// Any type `any`.
    Any,
    /// Infer placeholder `_`.
    Infer,
    /// Undefined type and value.
    Undefined,
    /// Unknown type.
    Unknown,
    // TODO #Architecture: 'dynamic' type that auto-casts (like unknown, but implicit, like any)?
    /// Object type (any non-primitive).
    Object,
    /// Void type.
    Void,
    /// Null type and value.
    Null,
    /// Primitive type.
    Primitive(PrimitiveType),
    /// Intrinsic type.
    Intrinsic(IntrinsicType),
    /// Scalar literal.
    ScalarLiteral(ScalarLiteral),
}

/// A TypeIntrinsic is a compiler-provided intrinsic type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, AdaptImage)]
pub enum IntrinsicType {
    /// Uppercase string intrinsic.
    Uppercase,
    /// Lowercase string intrinsic.
    Lowercase,
    /// Capitalize string intrinsic.
    Capitalize,
    /// Uncapitalize string intrinsic.
    Uncapitalize,
    /// NoInfer intrinsic.
    NoInfer,
    /// Builtin iterator return intrinsic.
    BuiltinIteratorReturn,
}

/// A mapped-type modifier in evaluated type space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, AdaptImage)]
pub enum MappedTypeModifier {
    /// The plain modifier without an explicit sign.
    Present,
    /// Add a modifier with an explicit `+` sign.
    Add,
    /// Remove a modifier (like `-readonly` or `-?`).
    Remove,
    /// No modifier specified.
    None,
}

/// Evaluated mapped-type modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, AdaptImage)]
pub struct MappedTypeModifiers {
    /// The readonly modifier.
    pub readonly: MappedTypeModifier,
    /// The optional modifier.
    pub optional: MappedTypeModifier,
}

/// An evaluated mapped-type parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub struct MappedTypeParameter {
    /// The parameter name (like `K`).
    pub name: StringId,
    /// The parameter symbol.
    pub symbol: GlobalSymbolId,
    /// The constraint type (like `keyof T`).
    pub constraint: LocalTypeId,
    /// The optional key remap (like `as Foo<K>`).
    pub key_remap: Option<LocalTypeId>,
}

/// A predicate subject in evaluated type space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, AdaptImage)]
pub enum PredicateSubject {
    /// Unresolved identifier subject (like `x` in `x is T`).
    Unresolved(StringId),
    /// Symbol subject (like `x` in `x is T`).
    Symbol(GlobalSymbolId),
    /// `this` subject.
    This,
}

/// An index signature in an object type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub struct TypeIndexSignature {
    /// The parameter name (like `K`).
    pub name: StringId,
    /// The key type (like `string`).
    pub key_type: LocalTypeId,
    /// The value type (like `T`).
    pub value_type: LocalTypeId,
    /// Whether the index signature is optional.
    pub is_optional: bool,
    /// Whether the index signature is readonly.
    pub is_readonly: bool,
}

/// A Type in the type system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum Type {
    /// Scalar type literal.
    TypeLiteral { value: TypeLiteral },

    /// Inference variable used during type analysis (should not be used outside of analysis).
    InferVar { id: InferVarId },

    /// Type-as-value: runtime representation of a type (for reflection and instanceof).
    Value { value: LocalTypeId },

    /// This type in a type predicate or method signature.
    This,

    /// Reference to a declared type (with optional type arguments for generics).
    Reference {
        symbol: GlobalSymbolId,
        static_arguments: Option<Vec<StaticArgument>>,
    },
    /// Unevaluated type expression that resolves to a type.
    Unevaluated(LocalNodeId<TypeExpression>),

    /// Type conditional expression.
    Conditional {
        distributive_symbol: Option<GlobalSymbolId>,
        left: LocalTypeId,
        right: LocalTypeId,
        then_type: LocalTypeId,
        else_type: LocalTypeId,
    },
    /// Type mapped expression.
    Mapped {
        parameter: MappedTypeParameter,
        modifiers: MappedTypeModifiers,
        value: LocalTypeId,
    },
    /// Type index expression.
    Index {
        left: LocalTypeId,
        index: LocalTypeId,
    },
    /// Type template literal expression.
    TemplateLiteral {
        strings: Vec<StringId>,
        spans: Vec<LocalTypeId>,
    },
    /// Type import expression.
    Import {
        target: StringId,
        qualifier: Option<Path>,
        static_arguments: Option<Vec<StaticArgument>>,
    },
    /// Type infer binding.
    Infer {
        name: StringId,
        constraint: Option<LocalTypeId>,
    },
    /// Type predicate expression.
    Predicate {
        asserts: bool,
        subject: PredicateSubject,
        target: Option<LocalTypeId>,
    },

    /// Type unary operator.
    Unary {
        operator: TypeUnaryOperator,
        right: LocalTypeId,
    },
    /// Value `^T` of a `T`. Or `^readonly T` for a readonly value.
    ValueOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        right: LocalTypeId,
    },
    /// Borrowed reference type `&T` or `&readonly T`.
    ReferenceOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        right: LocalTypeId,
    },
    /// Raw pointer type `*T` or `*readonly T`.
    PointerOf {
        mutability: Option<Mutability>,
        right: LocalTypeId,
    },
    /// Type binary operator.
    Binary {
        left: LocalTypeId,
        operator: TypeBinaryOperator,
        right: LocalTypeId,
    },

    /// Array type with fixed size (like `T[N]`).
    ArraySized {
        element: LocalTypeId,
        count: LocalTypeId,
        is_readonly: bool,
    },
    /// Array type with dynamically sized elements (like `T[]`).
    Array {
        element: Option<LocalTypeId>,
        is_readonly: bool,
    },
    /// Tuple type `(T1, T2, ...)`.
    Tuple {
        elements: Vec<TypeElement>,
        is_readonly: bool,
    },
    /// Object type `{ a: T1, b: T2, ... }`.
    Object {
        fields: Vec<TypeField>,
        call_signatures: Vec<LocalTypeId>,
        construct_signatures: Vec<LocalTypeId>,
        index_signatures: Vec<TypeIndexSignature>,
    },
    /// Function type `(T1, T2, ...) -> T`.
    Function {
        asynchrony: Asynchrony,
        cardinality: FunctionCardinality,
        static_parameters: Vec<LocalTypeId>,
        this_parameter: Option<LocalTypeId>,
        dynamic_parameters: Vec<LocalTypeId>,
        return_type: Option<LocalTypeId>,
    },

    // combinators
    /// Union type `A | B | C`.
    Union { elements: Vec<LocalTypeId> },
    /// Intersection type `A & B & C`.
    Intersection { elements: Vec<LocalTypeId> },

    /// Error type that could not be resolved.
    Error,
}

impl Type {
    /// Whether the type is evaluated.
    pub fn is_evaluated(&self) -> bool {
        !matches!(self, Type::Unevaluated(_) | Type::InferVar { .. })
    }

    /// Whether the type is unevaluated.
    pub fn is_unevaluated(&self) -> bool {
        matches!(self, Type::Unevaluated(_))
    }

    /// Whether the type is an error.
    pub fn is_error(&self) -> bool {
        matches!(self, Type::Error)
    }

    /// Whether the type is an unknown literal.
    pub fn is_unknown(&self) -> bool {
        matches!(
            self,
            Type::TypeLiteral {
                value: TypeLiteral::Unknown
            }
        )
    }

    /// Whether the type is one infer-owned placeholder variant.
    pub fn is_infer(&self) -> bool {
        matches!(self, Type::InferVar { .. } | Type::Infer { .. })
    }

    /// Whether the type is an error or unknown literal.
    pub fn is_error_or_unknown(&self) -> bool {
        self.is_error() || self.is_unknown()
    }

    /// Get the type symbol if this type is a direct reference to a declared type.
    pub fn symbol(&self) -> Option<GlobalSymbolId> {
        match self {
            Type::Reference { symbol, .. } => Some(*symbol),
            _ => None,
        }
    }
}

/// Type fields in an object-like type.
/// Methods are represented as fields whose `ty` is a `Type::Function`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub struct TypeField {
    /// The key of the field.
    pub key: StaticKey,
    /// The type of the field.
    pub ty: LocalTypeId,
    /// Whether the field is optional.
    pub is_optional: bool,
    /// Whether the field is readonly.
    pub is_readonly: bool,
}

/// A tuple element in a type tuple.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, AdaptImage)]
pub struct TypeElement {
    /// The optional label for the element.
    pub label: Option<StringId>,
    /// The element type.
    pub ty: LocalTypeId,
    /// Whether the element is optional.
    pub is_optional: bool,
    /// Whether the element is readonly.
    pub is_readonly: bool,
    /// Whether the element is a rest element.
    pub is_rest: bool,
}

impl TypeElement {
    /// Create a default tuple element for a type.
    pub fn new(ty: LocalTypeId) -> Self {
        Self {
            label: None,
            ty,
            is_optional: false,
            is_readonly: false,
            is_rest: false,
        }
    }
}

/// A TypeKind determines nominal vs. structural typing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, AdaptImage)]
pub enum TypeKind {
    /// Structural typing (like `type T = { a: int32, b: boolean }`).
    Structural,
    /// Nominal typing (like `newtype T = int32`).
    Nominal,
}

/// Unique identifier for Types.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, AdaptImage,
)]
pub struct LocalTypeId(pub u32);

impl LocalTypeId {
    /// Wrap an id as a TypeId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Turn into a GlobalTypeId.
    pub fn into_global(self, module_id: ModuleId) -> GlobalTypeId {
        GlobalTypeId {
            module_id,
            local_id: self,
        }
    }
}

/// Unique identifier for inference variables.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, AdaptImage,
)]
pub struct InferVarId(pub u32);

impl InferVarId {
    /// Wrap an id as an InferVarId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Global type id across modules.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, AdaptImage,
)]
pub struct GlobalTypeId {
    /// The module id of the global type.
    pub module_id: ModuleId,
    /// The local id of the global type.
    pub local_id: LocalTypeId,
}

impl GlobalTypeId {
    /// Create a new global type id.
    pub fn new(module_id: ModuleId, local_id: LocalTypeId) -> Self {
        Self {
            module_id,
            local_id,
        }
    }

    /// Turn into a LocalTypeId.
    pub fn into_local(self) -> LocalTypeId {
        self.local_id
    }
}

impl From<GlobalTypeId> for LocalTypeId {
    fn from(id: GlobalTypeId) -> Self {
        id.local_id
    }
}
