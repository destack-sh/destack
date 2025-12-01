use crate::{
    Asynchrony, BindingModifier, Expression, FunctionCardinality, FunctionSignature, Key,
    LocalNodeId, LocalSymbolId, Mutability, ScalarLiteral, VarianceBound,
};

use super::{DeclarationType, PrimitiveType, TypeBinaryOperator, TypeUnaryOperator};

use crate::ModuleId;

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
    Composite(DeclarationType),
    /// Scalar literal.
    ScalarLiteral(ScalarLiteral),
}

/// An Type in the type system.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Scalar type literal.
    Scalar(TypeLiteral),
    /// Redirect to symbol type.
    Symbol(LocalSymbolId),
    /// Unresolved expression type.
    Unresolved(LocalNodeId<Expression>),

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
    /// Reference of `&T` to a `T`. Or `&var T` for a mutable reference.
    ReferenceOf {
        mutability: Option<Mutability>,
        variance: Option<VarianceBound>,
        right: LocalTypeId,
    },
    /// Type binary operator.
    Binary {
        left: LocalTypeId,
        operator: TypeBinaryOperator,
        right: LocalTypeId,
    },

    /// Range type `T..T` (or `T..=T` for inclusive range).
    Range {
        start: LocalTypeId,
        end: LocalTypeId,
        is_inclusive: bool,
    },
    /// Array type with fixed size (like `T[N]`).
    ArraySized {
        element: LocalTypeId,
        count: LocalNodeId<Expression>,
    },
    /// Array type with dynamically sized elements (like `T[]`).
    Array { element: Option<LocalTypeId> },
    /// Tuple type `[T1, T2, ...]`.
    Tuple { elements: Vec<LocalTypeId> },
    /// Struct type `{ a: T1, b: T2, ... }`.
    Struct { fields: Vec<TypeField> },
    /// Union type `A | B | C`.
    Union { elements: Vec<LocalTypeId> },
    /// Intersection type `A & B & C`.
    Intersection { elements: Vec<LocalTypeId> },
    /// Function type `(T1, T2, ...) -> T`.
    Function {
        asynchrony: Asynchrony,
        cardinality: FunctionCardinality,
        static_parameters: Vec<LocalTypeId>,
        dynamic_parameters: Vec<LocalTypeId>,
        return_type: Option<LocalTypeId>,
    },

    /// Error type that could not be resolved.
    Error,
}

/// The type of an attribute (like a property or field).
#[derive(Debug, Clone, PartialEq)]
pub enum TypeField {
    /// Named field (like `a: T`).
    Field {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
    },
    /// Named method (like `foo(): T`).
    Method {
        modifiers: Option<BindingModifier>,
        key: Option<Key>,
        signature: FunctionSignature,
    },
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
