use destack_source::ModuleId;

use crate::{
    Asynchrony, Expression, FunctionCardinality, GlobalSymbolId, LocalNodeId, Mutability, Path,
    ScalarLiteral, StaticArgument, StaticKey, StringId, VarianceBound,
};

use super::{DeclarationType, PrimitiveType, TypeBinaryOperator, TypeUnaryOperator};

/// A TypeLiteral is a scalar type.
#[derive(Debug, Clone, PartialEq)]
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
    /// Void type.
    Void,
    /// Null type and value.
    Null,
    /// Primitive type.
    Primitive(PrimitiveType),
    /// Composite type.
    Composite(DeclarationType), // FUGU: remove TypeLiteral::Composite + DeclarationType?
    /// Intrinsic type (TypeScript compiler-provided).
    Intrinsic(TypeIntrinsic),
    /// Scalar literal.
    ScalarLiteral(ScalarLiteral),
}

/// A TypeIntrinsic is a compiler-provided intrinsic type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeIntrinsic {
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

/// A type modifier for mapped types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeModifier {
    /// Add a modifier (like `readonly` or `?`).
    Add,
    /// Remove a modifier (like `-readonly` or `-?`).
    Remove,
    /// No modifier specified.
    None,
}

/// Mapped type modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeMappedModifiers {
    /// The readonly modifier.
    pub readonly: TypeModifier,
    /// The optional modifier.
    pub optional: TypeModifier,
}

/// A mapped type parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeMappedParameter {
    /// The parameter name (like `K`).
    pub name: StringId,
    /// The constraint type (like `keyof T`).
    pub constraint: LocalTypeId,
    /// The optional key remap (like `as Foo<K>`).
    pub key_remap: Option<LocalTypeId>,
}

/// A type predicate subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypePredicateSubject {
    /// Unresolved identifier subject (like `x` in `x is T`).
    Unresolved(StringId),
    /// Symbol subject (like `x` in `x is T`).
    Symbol(GlobalSymbolId),
    /// `this` subject.
    This,
}

/// An index signature in an object type.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeIndexSignature {
    /// The parameter name (like `K`).
    pub name: StringId,
    /// The key type (like `string`).
    pub key_type: LocalTypeId,
    /// The value type (like `T`).
    pub value_type: LocalTypeId,
    /// Whether the index signature is readonly.
    pub is_readonly: bool,
}

/// A Type in the type system.
#[derive(Debug, Clone, PartialEq)]
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
    /// Unevaluated expression that resolves to a type (needs compile-time evaluation).
    Unevaluated(LocalNodeId<Expression>),

    /// Type conditional expression.
    Conditional {
        left: LocalTypeId,
        right: LocalTypeId,
        then_type: LocalTypeId,
        else_type: LocalTypeId,
    },
    /// Type mapped expression.
    Mapped {
        parameter: TypeMappedParameter,
        modifiers: TypeMappedModifiers,
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
        subject: TypePredicateSubject,
        target: Option<LocalTypeId>,
    },

    /// Type unary operator.
    Unary {
        operator: TypeUnaryOperator,
        right: LocalTypeId,
    },
    /// Mutable or immutable type `T`.
    Mutable {
        mutability: Mutability,
        right: LocalTypeId,
    },
    /// Value `^T` of a `T`. Or `^var T` for a mutable value.
    ValueOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        right: LocalTypeId,
    },
    /// Reference of `&T` to a `T`. Or `&mut T` for a mutable reference.
    ReferenceOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        right: LocalTypeId,
    },
    /// Pointer type `*T` to a `T` or `*mut T` for a mutable pointer.
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
        count: LocalNodeId<Expression>,
    },
    /// Array type with dynamically sized elements (like `T[]`).
    Array { element: Option<LocalTypeId> },
    /// Tuple type `(T1, T2, ...)`.
    Tuple { elements: Vec<TypeElement> },
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
        !matches!(self, Type::Unevaluated { .. } | Type::InferVar { .. })
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
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TypeKind {
    /// Structural typing (like `type T = { a: int32, b: boolean }`).
    Structural,
    /// Nominal typing (like `newtype T = int32`).
    Nominal,
}

/// Unique identifier for Types.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InferVarId(pub u32);

impl InferVarId {
    /// Wrap an id as an InferVarId.
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Global type id across modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
