use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::{LocalNodeId, Node, NodeType, TypeReference};

/// Borrow-region bounds for a returned borrowed value.
///
/// Specifies which function parameters a returned reference (or aggregate
/// containing references) may borrow from. Used by the borrow checker to
/// track borrows across function calls.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BorrowRegion {
    /// Infer lifetime based on function signature:
    /// - Single `&T` parameter: return borrows from it
    /// - `&self`/`&this` receiver: return borrows from receiver
    /// - Multiple `&T` parameters: conservative (borrows from all)
    #[default]
    Inferred,
    /// Borrows from specific parameters (by index).
    /// E.g., `@lifetime(a, b)` where a is param 0 and b is param 1.
    /// Typically 1-2 parameters, so Vec is efficient enough.
    Parameters(Vec<u32>),
    /// Static lifetime - borrows only from global/static data.
    /// The returned reference is valid for the entire program lifetime.
    Static,
}

impl BorrowRegion {
    /// Create a borrow region bound for a single parameter.
    pub fn param(index: u32) -> Self {
        BorrowRegion::Parameters(vec![index])
    }

    /// Create a borrow region bound for multiple parameters.
    pub fn params(indices: impl IntoIterator<Item = u32>) -> Self {
        BorrowRegion::Parameters(indices.into_iter().collect())
    }

    /// Check if this is the inferred default borrow region.
    pub fn is_inferred(&self) -> bool {
        matches!(self, BorrowRegion::Inferred)
    }

    /// Check if this is a static borrow region.
    pub fn is_static(&self) -> bool {
        matches!(self, BorrowRegion::Static)
    }

    /// Check if this borrow region includes a specific parameter.
    pub fn includes_param(&self, index: u32) -> bool {
        match self {
            BorrowRegion::Parameters(params) => params.contains(&index),
            _ => false,
        }
    }
}

/// Mutability of a reference or binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mutability {
    /// Immutable (const).
    Immutable,
    /// Mutable (var).
    Mutable,
}

/// Address space for a reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AddressSpace {
    /// Local runtime storage.
    #[default]
    Local,
    /// Shared runtime storage.
    Shared,
    /// Stack or function-local memory.
    Stack,
    /// Frame-slot storage inside one activation.
    Frame,
    /// Global or module-static memory.
    Global,
    /// Named backend-specific storage space.
    Named(String),
}

impl AddressSpace {
    /// Check if this is the local runtime storage space.
    pub fn is_local(&self) -> bool {
        matches!(self, AddressSpace::Local)
    }

    /// Parse a named address space.
    pub fn from_name(name: &str) -> Self {
        match name {
            "local" => AddressSpace::Local,
            "shared" => AddressSpace::Shared,
            "stack" => AddressSpace::Stack,
            "frame" => AddressSpace::Frame,
            "global" => AddressSpace::Global,
            _ => AddressSpace::Named(name.to_string()),
        }
    }

    /// Return the canonical source name for this address space.
    pub fn label(&self) -> &str {
        match self {
            AddressSpace::Local => "local",
            AddressSpace::Shared => "shared",
            AddressSpace::Stack => "stack",
            AddressSpace::Frame => "frame",
            AddressSpace::Global => "global",
            AddressSpace::Named(name) => name.as_str(),
        }
    }
}

/// Kind of reference in MIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl ReferenceKind {
    /// Whether this reference kind is affine.
    pub fn is_affine(self) -> bool {
        matches!(self, ReferenceKind::Owned)
    }
}

/// Copyability of a type.
///
/// Determines whether values of this type can be used multiple times
/// or if each use consumes the value (linear/move semantics).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Copyability {
    /// Value can be used multiple times freely (like Copy in Rust).
    /// This is the default for primitives and non-owning references.
    #[default]
    Trivial,
    /// Each use consumes the value (linear/move-only).
    /// Required for affine ownership and aggregates containing affine ownership.
    Linear,
}

impl Copyability {
    /// Whether this is trivially copyable.
    pub fn is_trivial(&self) -> bool {
        matches!(self, Copyability::Trivial)
    }

    /// Whether this has linear/move semantics.
    pub fn is_linear(&self) -> bool {
        matches!(self, Copyability::Linear)
    }

    /// Combine two copyabilities (for aggregate types).
    /// Returns Linear if either is Linear, otherwise Trivial.
    pub fn combine(self, other: Copyability) -> Copyability {
        match (self, other) {
            (Copyability::Trivial, Copyability::Trivial) => Copyability::Trivial,
            _ => Copyability::Linear,
        }
    }
}

/// Layout for a tensor or tensor reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TensorLayout {
    /// Contiguous row-major layout.
    RowMajor,
    /// Contiguous column-major layout.
    ColumnMajor,
    /// Explicit strided layout.
    Strided {
        /// Strides for each dimension in element units.
        strides: Vec<TensorDimension>,
    },
}

/// Dimension size for tensor shapes and layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TensorDimension {
    /// Compile time static dimension size.
    Static(u64),
    /// Runtime dynamic dimension size.
    Dynamic,
}

impl TensorDimension {
    /// Check whether this dimension is dynamic.
    pub fn is_dynamic(self) -> bool {
        matches!(self, TensorDimension::Dynamic)
    }
}

/// Concrete type in MIR (post-monomorphization).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    /// Void / unit type (no value).
    Void,
    /// Boolean (1 bit logical, typically 1 byte).
    Boolean,
    /// Integer with explicit width and signedness.
    Int { width: u16, is_signed: bool },
    /// Pointer-sized signed integer.
    Isize,
    /// Pointer-sized unsigned integer.
    Usize,
    /// Floating point with explicit width (32 or 64).
    Float { width: u16 },
    /// Runtime type descriptor handle.
    TypeDescriptor,
    /// Compact runtime type identity token.
    TypeId,

    /// Reference with explicit kind and mutability.
    Reference {
        /// The reference kind (managed, owned, borrowed, raw).
        kind: ReferenceKind,
        /// The address space for this reference.
        address_space: AddressSpace,
        /// The reference mutability.
        mutability: Mutability,
        /// The referenced type.
        pointee: TypeReference,
        /// Whether the reference can be null.
        is_nullable: bool,
    },

    /// Fixed-size array: `T[N]`.
    Array {
        /// The element type of the array.
        element: TypeReference,
        /// The number of elements in the array.
        length: u64,
        /// Copyability of this array type.
        copyability: Copyability,
    },
    /// Dynamic array: `T[]`.
    DynamicArray {
        /// The element type of the array.
        element: TypeReference,
        /// Copyability of this array type.
        copyability: Copyability,
    },
    /// Tuple: `(T1, T2, ...)`.
    Tuple {
        /// The element types of the tuple.
        elements: Vec<TypeReference>,
        /// Copyability of this tuple type.
        copyability: Copyability,
    },
    /// Struct (anonymous, layout-focused).
    Struct {
        /// The fields of the struct.
        fields: Vec<LocalNodeId<Field>>,
        /// Copyability of this struct type.
        copyability: Copyability,
    },
    /// Nominal newtype wrapping an inner type.
    Newtype {
        /// The wrapped inner type.
        inner: TypeReference,
        /// Copyability of this newtype.
        copyability: Copyability,
    },

    /// Fixed-width SIMD vector.
    Vector {
        /// The element type.
        element: TypeReference,
        /// The number of lanes.
        lanes: u32,
        /// Copyability of this vector type.
        copyability: Copyability,
    },
    /// Ranked tensor value with static or dynamic shape.
    Tensor {
        /// The element type.
        element: TypeReference,
        /// The static shape.
        shape: Vec<TensorDimension>,
        /// The tensor layout.
        layout: TensorLayout,
        /// Copyability of this tensor type.
        copyability: Copyability,
    },
    /// Reference-like view into tensor-shaped memory.
    TensorReference {
        /// The reference kind (managed, owned, borrowed, raw).
        kind: ReferenceKind,
        /// The address space for this view.
        address_space: AddressSpace,
        /// The view mutability.
        mutability: Mutability,
        /// The element type.
        element: TypeReference,
        /// The static shape.
        shape: Vec<TensorDimension>,
        /// The tensor layout.
        layout: TensorLayout,
        /// Whether the view can be null.
        is_nullable: bool,
    },

    /// Function pointer type.
    FunctionPointer {
        /// The parameters of the function.
        parameters: Vec<TypeReference>,
        /// The result type of the function.
        result: TypeReference,
    },
    /// Callable closure value with code and environment.
    Closure {
        /// The bare function pointer signature.
        signature: TypeReference,
    },
}

impl Node for Type {
    const TYPE: NodeType = NodeType::Type;
}

impl Type {
    pub const INT8: Type = Type::Int {
        width: 8,
        is_signed: true,
    };
    pub const INT16: Type = Type::Int {
        width: 16,
        is_signed: true,
    };
    pub const INT32: Type = Type::Int {
        width: 32,
        is_signed: true,
    };
    pub const INT64: Type = Type::Int {
        width: 64,
        is_signed: true,
    };
    pub const INT128: Type = Type::Int {
        width: 128,
        is_signed: true,
    };
    pub const INT256: Type = Type::Int {
        width: 256,
        is_signed: true,
    };

    pub const UINT8: Type = Type::Int {
        width: 8,
        is_signed: false,
    };
    pub const UINT16: Type = Type::Int {
        width: 16,
        is_signed: false,
    };
    pub const UINT32: Type = Type::Int {
        width: 32,
        is_signed: false,
    };
    pub const UINT64: Type = Type::Int {
        width: 64,
        is_signed: false,
    };
    pub const UINT128: Type = Type::Int {
        width: 128,
        is_signed: false,
    };
    pub const UINT256: Type = Type::Int {
        width: 256,
        is_signed: false,
    };

    pub const FLOAT32: Type = Type::Float { width: 32 };
    pub const FLOAT64: Type = Type::Float { width: 64 };

    /// Return integer width and signedness for concrete integer types.
    pub fn int_info(&self) -> Option<(u16, bool)> {
        match self {
            Type::Int {
                width,
                is_signed: signed,
            } => Some((*width, *signed)),
            _ => None,
        }
    }

    /// Return integer width and signedness with pointer-sized integers resolved.
    pub fn int_info_with_pointer_width(&self, pointer_width_bits: u16) -> Option<(u16, bool)> {
        match self {
            Type::Int {
                width,
                is_signed: signed,
            } => Some((*width, *signed)),
            Type::Isize => Some((pointer_width_bits, true)),
            Type::Usize => Some((pointer_width_bits, false)),
            _ => None,
        }
    }

    /// Whether this type is a scalar (non-compound).
    pub fn is_scalar(&self) -> bool {
        matches!(
            self,
            Type::Void
                | Type::Boolean
                | Type::Int { .. }
                | Type::Isize
                | Type::Usize
                | Type::Float { .. }
                | Type::TypeDescriptor
                | Type::TypeId
                | Type::Reference { .. }
                | Type::Vector { .. }
                | Type::TensorReference { .. }
        )
    }

    /// Whether this type is a raw pointer.
    pub fn is_raw_pointer(&self) -> bool {
        matches!(
            self,
            Type::Reference {
                kind: ReferenceKind::Raw,
                ..
            } | Type::TensorReference {
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
            } | Type::TensorReference {
                kind: ReferenceKind::Managed,
                ..
            }
        )
    }

    /// Whether this type is any kind of pointer or reference.
    pub fn is_pointer_like(&self) -> bool {
        matches!(self, Type::Reference { .. } | Type::TensorReference { .. })
    }

    /// Whether this type is a borrowed reference.
    pub fn is_borrowed_reference(&self) -> bool {
        matches!(
            self,
            Type::Reference {
                kind: ReferenceKind::Borrowed,
                ..
            } | Type::TensorReference {
                kind: ReferenceKind::Borrowed,
                ..
            }
        )
    }

    /// Whether this type is a mutable borrowed reference.
    pub fn is_mutable_borrowed_reference(&self) -> bool {
        matches!(
            self,
            Type::Reference {
                kind: ReferenceKind::Borrowed,
                mutability: Mutability::Mutable,
                ..
            } | Type::TensorReference {
                kind: ReferenceKind::Borrowed,
                mutability: Mutability::Mutable,
                ..
            }
        )
    }

    /// Get the copyability of this type.
    ///
    /// - Primitives (void, bool, int, float) are always Trivial
    /// - Non-owning references (borrowed, raw) are Trivial
    /// - Affine references (owned) are Linear
    /// - Aggregates have explicit copyability stored in their variants
    /// - Function pointers are Trivial
    pub fn copyability(&self) -> Copyability {
        match self {
            // primitives are always trivially copyable
            Type::Void
            | Type::Boolean
            | Type::Int { .. }
            | Type::Isize
            | Type::Usize
            | Type::Float { .. }
            | Type::TypeDescriptor
            | Type::TypeId => Copyability::Trivial,

            // references depend on ownership
            Type::Reference { kind, .. } => {
                if kind.is_affine() {
                    Copyability::Linear
                } else {
                    Copyability::Trivial
                }
            }

            // aggregates have explicit copyability
            Type::Array { copyability, .. }
            | Type::DynamicArray { copyability, .. }
            | Type::Tuple { copyability, .. }
            | Type::Struct { copyability, .. }
            | Type::Newtype { copyability, .. }
            | Type::Vector { copyability, .. }
            | Type::Tensor { copyability, .. } => *copyability,

            // tensor references behave like references
            Type::TensorReference { kind, .. } => {
                if kind.is_affine() {
                    Copyability::Linear
                } else {
                    Copyability::Trivial
                }
            }

            // function pointers and closure values are trivially copyable
            Type::FunctionPointer { .. } | Type::Closure { .. } => Copyability::Trivial,
        }
    }
}

/// A field in a struct type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    /// Name (optional).
    pub name: Option<StringId>,
    /// Type of the field.
    pub ty: TypeReference,
}

impl Node for Field {
    const TYPE: NodeType = NodeType::Field;
}

/// A named type alias in MIR text format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeAlias {
    /// Alias name (without the leading `@`).
    pub name: StringId,
    /// The aliased type.
    pub ty: TypeReference,
}

impl Node for TypeAlias {
    const TYPE: NodeType = NodeType::TypeAlias;
}
