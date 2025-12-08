use destack_source::StringId;

use crate::{LocalNodeId, Node, NodeType};

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

    /// Raw pointer (manual memory management).
    /// The caller is responsible for allocation and deallocation.
    RawPointer { pointee: LocalNodeId<Type> },

    /// Managed reference (runtime-tracked lifetime).
    /// The runtime (GC, refcount, arena, etc.) manages the memory.
    ManagedReference {
        pointee: LocalNodeId<Type>,
        /// Whether the reference can be null.
        nullable: bool,
    },

    /// Fixed-size array: `T[N]`.
    Array {
        element: LocalNodeId<Type>,
        length: u64,
    },
    /// Tuple: `(T1, T2, ...)`.
    Tuple { elements: Vec<LocalNodeId<Type>> },
    /// Struct (anonymous, layout-focused).
    Struct { fields: Vec<Field> },

    /// Function pointer type.
    FunctionPointer {
        parameters: Vec<LocalNodeId<Type>>,
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
                | Type::RawPointer { .. }
                | Type::ManagedReference { .. }
        )
    }

    /// Whether this type is a raw pointer.
    pub fn is_raw_pointer(&self) -> bool {
        matches!(self, Type::RawPointer { .. })
    }

    /// Whether this type is a managed reference.
    pub fn is_managed_reference(&self) -> bool {
        matches!(self, Type::ManagedReference { .. })
    }

    /// Whether this type is any kind of pointer or reference.
    pub fn is_pointer_like(&self) -> bool {
        matches!(
            self,
            Type::RawPointer { .. } | Type::ManagedReference { .. }
        )
    }
}

/// A field in a struct type.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// Optional name (for debugging / error messages).
    pub name: Option<StringId>,
    /// Type of the field.
    pub ty: LocalNodeId<Type>,
    /// Byte offset within the struct.
    pub offset: u32,
}
