use destack_base::StringId;

use crate::{LocalNodeId, Node, NodeType};

/// Mutability of a reference or binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mutability {
    /// Immutable (const).
    Immutable,
    /// Mutable (var).
    Mutable,
}

/// Kind of reference in MIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReferenceKind {
    /// GC-managed reference.
    Managed,
    /// Explicit ownership reference.
    Owned,
    /// Borrowed reference.
    Borrowed,
    /// Raw pointer reference.
    Raw,
}

/// Concrete type in MIR (post-monomorphization).
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Void / unit type (no value).
    Void,
    /// Boolean (1 bit logical, typically 1 byte).
    Boolean,
    /// Integer with explicit width and signedness.
    Int { width: u16, signed: bool },
    /// Floating point with explicit width (32 or 64).
    Float { width: u16 },

    /// Reference with explicit kind and mutability.
    Reference {
        /// The reference kind (managed, owned, borrowed, raw).
        kind: ReferenceKind,
        /// The reference mutability.
        mutability: Mutability,
        /// The referenced type.
        pointee: LocalNodeId<Type>,
        /// Whether the reference can be null.
        is_nullable: bool,
    },

    /// Fixed-size array: `T[N]`.
    Array {
        /// The element type of the array.
        element: LocalNodeId<Type>,
        /// The number of elements in the array.
        length: u64,
    },
    /// Tuple: `(T1, T2, ...)`.
    Tuple { elements: Vec<LocalNodeId<Type>> },
    /// Struct (anonymous, layout-focused).
    Struct {
        /// The fields of the struct.
        fields: Vec<LocalNodeId<Field>>,
    },

    /// Function pointer type.
    FunctionPointer {
        /// The parameters of the function.
        parameters: Vec<LocalNodeId<Type>>,
        /// The result type of the function.
        result: LocalNodeId<Type>,
    },
}

impl Node for Type {
    const TYPE: NodeType = NodeType::Type;
}

impl Type {
    pub const INT8: Type = Type::Int {
        width: 8,
        signed: true,
    };
    pub const INT16: Type = Type::Int {
        width: 16,
        signed: true,
    };
    pub const INT32: Type = Type::Int {
        width: 32,
        signed: true,
    };
    pub const INT64: Type = Type::Int {
        width: 64,
        signed: true,
    };
    pub const INT128: Type = Type::Int {
        width: 128,
        signed: true,
    };
    pub const INT256: Type = Type::Int {
        width: 256,
        signed: true,
    };

    pub const UINT8: Type = Type::Int {
        width: 8,
        signed: false,
    };
    pub const UINT16: Type = Type::Int {
        width: 16,
        signed: false,
    };
    pub const UINT32: Type = Type::Int {
        width: 32,
        signed: false,
    };
    pub const UINT64: Type = Type::Int {
        width: 64,
        signed: false,
    };
    pub const UINT128: Type = Type::Int {
        width: 128,
        signed: false,
    };
    pub const UINT256: Type = Type::Int {
        width: 256,
        signed: false,
    };

    pub const FLOAT32: Type = Type::Float { width: 32 };
    pub const FLOAT64: Type = Type::Float { width: 64 };

    /// Whether this type is a scalar (non-compound).
    pub fn is_scalar(&self) -> bool {
        matches!(
            self,
            Type::Void
                | Type::Boolean
                | Type::Int { .. }
                | Type::Float { .. }
                | Type::Reference { .. }
        )
    }

    /// Whether this type is a raw pointer.
    pub fn is_raw_pointer(&self) -> bool {
        matches!(
            self,
            Type::Reference {
                kind: ReferenceKind::Raw,
                ..
            }
        )
    }

    /// Whether this type is a managed reference.
    pub fn is_managed_reference(&self) -> bool {
        matches!(
            self,
            Type::Reference {
                kind: ReferenceKind::Managed,
                ..
            }
        )
    }

    /// Whether this type is any kind of pointer or reference.
    pub fn is_pointer_like(&self) -> bool {
        matches!(self, Type::Reference { .. })
    }
}

/// A field in a struct type.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// Name (optional).
    pub name: Option<StringId>,
    /// Type of the field.
    pub ty: LocalNodeId<Type>,
    /// Byte offset within the struct.
    pub offset: u32,
}

impl Node for Field {
    const TYPE: NodeType = NodeType::Field;
}

/// A named type alias in MIR text format.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeAlias {
    /// Alias name (without the leading `@`).
    pub name: StringId,
    /// The aliased type.
    pub ty: LocalNodeId<Type>,
}

impl Node for TypeAlias {
    const TYPE: NodeType = NodeType::TypeAlias;
}
