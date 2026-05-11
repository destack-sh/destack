use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::{Lifetime, LocalNodeId, Node, NodeType, TypeReference};

/// Mutability of a storage binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mutability {
    /// Immutable (const).
    Immutable,
    /// Mutable (var).
    Mutable,
}

/// Access exposed by a reference-like value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Access {
    /// Readonly access.
    Readonly,
    /// Mutable aliased access.
    #[default]
    Mutable,
    /// Mutable exclusive access.
    Exclusive,
}

impl Access {
    /// Return true when this access can write through the reference.
    pub fn can_write(self) -> bool {
        matches!(self, Access::Mutable | Access::Exclusive)
    }

    /// Return true when this access excludes overlapping borrows.
    pub fn is_exclusive(self) -> bool {
        matches!(self, Access::Exclusive)
    }
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
    /// Static memory.
    Static,
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
            "static" => AddressSpace::Static,
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
            AddressSpace::Static => "static",
            AddressSpace::Named(name) => name.as_str(),
        }
    }
}

/// Kind of reference in MIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReferenceKind {
    /// GC-managed reference.
    Managed,
    /// Unique typed heap reference.
    Unique,
    /// Borrowed reference.
    Borrowed,
    /// Raw pointer reference.
    Raw,
}

/// Copy property of a type.
///
/// Determines whether values of this type can be duplicated freely
/// or if each use consumes the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Copy {
    /// Value can be copied freely.
    #[default]
    Yes,
    /// Each use consumes the value.
    No,
}

impl Copy {
    /// Report whether this type can be copied freely.
    pub fn is_yes(self) -> bool {
        matches!(self, Copy::Yes)
    }

    /// Report whether this type is move only.
    pub fn is_no(self) -> bool {
        matches!(self, Copy::No)
    }

    /// Combine two copy properties for an aggregate type.
    pub fn combine(self, other: Copy) -> Copy {
        match (self, other) {
            (Copy::Yes, Copy::Yes) => Copy::Yes,
            _ => Copy::No,
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
    /// Floating point with concrete representation.
    Float(FloatType),
    /// Runtime type descriptor handle.
    TypeDescriptor,
    /// Compact runtime type identity token.
    TypeId,

    /// Atomic storage cell for one value type.
    Atomic {
        /// The stored value type.
        value: TypeReference,
    },
    /// Runtime-erased value satisfying one lowered interface contract.
    Any {
        /// The lowered interface contract type.
        interface: TypeReference,
    },
    /// Reference with explicit kind and access.
    Reference {
        /// The reference kind (managed, unique, borrowed, raw).
        kind: ReferenceKind,
        /// Lifetime roots for borrowed references.
        lifetime: Lifetime,
        /// The address space for this reference.
        address_space: AddressSpace,
        /// The access exposed through this reference.
        access: Access,
        /// The referenced type.
        pointee: TypeReference,
        /// Whether the reference can be null.
        is_nullable: bool,
    },
    /// Slice into memory, a repeated element with explicit kind and access.
    Slice {
        /// The reference kind of the slice base.
        kind: ReferenceKind,
        /// Lifetime roots for borrowed slices.
        lifetime: Lifetime,
        /// The element type of the slice.
        element: TypeReference,
        /// The address space of the slice base.
        address_space: AddressSpace,
        /// The element access exposed by the slice.
        access: Access,
    },

    /// Fixed-size array: `T[N]`.
    Array {
        /// The element type of the array.
        element: TypeReference,
        /// The number of elements in the array.
        length: u64,
        /// Copy of this array type.
        copy: Copy,
    },
    /// Tuple: `(T1, T2, ...)`.
    Tuple {
        /// The element types of the tuple.
        elements: Vec<TypeReference>,
        /// Copy of this tuple type.
        copy: Copy,
    },
    /// Struct (anonymous, layout-focused).
    Struct {
        /// The fields of the struct.
        fields: Vec<LocalNodeId<Field>>,
        /// Copy of this struct type.
        copy: Copy,
    },
    /// Nominal newtype wrapping an inner type.
    Newtype {
        /// The wrapped inner type.
        inner: TypeReference,
        /// Copy of this newtype.
        copy: Copy,
    },
    /// Tagged union value.
    Union {
        /// The tag value type.
        tag: TypeReference,
        /// The variants keyed by tag value.
        variants: Vec<UnionVariant>,
        /// Copy of this union type.
        copy: Copy,
    },

    /// Fixed-width SIMD vector.
    Vector {
        /// The element type.
        element: TypeReference,
        /// The number of lanes.
        lanes: u32,
        /// Copy of this vector type.
        copy: Copy,
    },
    /// Ranked tensor value with static or dynamic shape.
    Tensor {
        /// The element type.
        element: TypeReference,
        /// The static shape.
        shape: Vec<TensorDimension>,
        /// The tensor layout.
        layout: TensorLayout,
        /// Copy of this tensor type.
        copy: Copy,
    },
    /// Reference-like view into tensor-shaped memory.
    TensorView {
        /// The reference kind (managed, unique, borrowed, raw).
        kind: ReferenceKind,
        /// Lifetime roots for borrowed tensor views.
        lifetime: Lifetime,
        /// The address space for this view.
        address_space: AddressSpace,
        /// The access exposed through this view.
        access: Access,
        /// The element type.
        element: TypeReference,
        /// The static shape.
        shape: Vec<TensorDimension>,
        /// The tensor layout.
        layout: TensorLayout,
        /// Whether the view can be null.
        is_nullable: bool,
    },

    /// Bare function signature.
    FunctionSignature {
        /// The parameters of the function.
        parameters: Vec<TypeReference>,
        /// The result type of the function.
        result: TypeReference,
    },
    /// Function pointer type.
    FunctionPointer {
        /// The bare function signature.
        signature: TypeReference,
    },
    /// Opaque callable value with code and environment.
    Callable {
        /// The bare function signature.
        signature: TypeReference,
    },
}

/// One tagged union variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnionVariant {
    /// The numeric tag value selecting this variant.
    pub tag: u64,
    /// The variant value type.
    pub ty: TypeReference,
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

    pub const FLOAT32: Type = Type::Float(FloatType::Float32);
    pub const FLOAT64: Type = Type::Float(FloatType::Float64);

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
                | Type::Float(_)
                | Type::TypeDescriptor
                | Type::TypeId
                | Type::Reference { .. }
                | Type::Vector { .. }
                | Type::TensorView { .. }
        )
    }

    /// Whether this type is a raw pointer.
    pub fn is_raw_pointer(&self) -> bool {
        matches!(
            self,
            Type::Reference {
                kind: ReferenceKind::Raw,
                ..
            } | Type::TensorView {
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
            } | Type::TensorView {
                kind: ReferenceKind::Managed,
                ..
            }
        )
    }

    /// Whether this type is any kind of pointer or reference.
    pub fn is_pointer_like(&self) -> bool {
        matches!(
            self,
            Type::Reference { .. } | Type::TensorView { .. } | Type::Slice { .. }
        )
    }

    /// Whether this type is a borrowed reference.
    pub fn is_borrowed_reference(&self) -> bool {
        matches!(
            self,
            Type::Reference {
                kind: ReferenceKind::Borrowed,
                ..
            } | Type::TensorView {
                kind: ReferenceKind::Borrowed,
                ..
            }
        )
    }

    /// Whether this type is an exclusive borrowed reference.
    pub fn is_exclusive_borrowed_reference(&self) -> bool {
        matches!(
            self,
            Type::Reference {
                kind: ReferenceKind::Borrowed,
                access: Access::Exclusive,
                ..
            } | Type::TensorView {
                kind: ReferenceKind::Borrowed,
                access: Access::Exclusive,
                ..
            }
        )
    }

    /// Return the copy property of this type.
    ///
    /// - Primitives are always copyable
    /// - Non-owning references are always copyable
    /// - Affine references are move only
    /// - Aggregates store their copy property explicitly
    /// - Function pointers are always copyable
    pub fn copy(&self) -> Copy {
        match self {
            // primitives are always trivially copyable
            Type::Void
            | Type::Boolean
            | Type::Int { .. }
            | Type::Isize
            | Type::Usize
            | Type::Float { .. }
            | Type::TypeDescriptor
            | Type::TypeId => Copy::Yes,

            // atomic cells are storage, not freely copied values
            Type::Atomic { .. } => Copy::No,

            // erased values may own hidden payloads
            Type::Any { .. } => Copy::No,

            // unique references carry ownership of typed heap storage
            Type::Reference { kind, .. } => match kind {
                ReferenceKind::Unique => Copy::No,
                ReferenceKind::Managed | ReferenceKind::Borrowed | ReferenceKind::Raw => Copy::Yes,
            },

            // unique slices carry ownership of typed heap storage
            Type::Slice { kind, .. } => match kind {
                ReferenceKind::Unique => Copy::No,
                ReferenceKind::Managed | ReferenceKind::Borrowed | ReferenceKind::Raw => Copy::Yes,
            },

            // aggregates have explicit copy
            Type::Array { copy, .. }
            | Type::Tuple { copy, .. }
            | Type::Struct { copy, .. }
            | Type::Newtype { copy, .. }
            | Type::Union { copy, .. }
            | Type::Vector { copy, .. }
            | Type::Tensor { copy, .. } => *copy,

            // unique tensor views carry ownership of typed heap storage
            Type::TensorView { kind, .. } => match kind {
                ReferenceKind::Unique => Copy::No,
                ReferenceKind::Managed | ReferenceKind::Borrowed | ReferenceKind::Raw => Copy::Yes,
            },

            // callable metadata and values are trivially copyable
            Type::FunctionSignature { .. }
            | Type::FunctionPointer { .. }
            | Type::Callable { .. } => Copy::Yes,
        }
    }
}

/// A concrete MIR floating-point type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FloatType {
    /// A 32-bit IEEE-754 float.
    Float32,
    /// A 64-bit IEEE-754 float.
    Float64,
}

impl FloatType {
    /// Return the bit width.
    pub fn width(self) -> u16 {
        match self {
            FloatType::Float32 => 32,
            FloatType::Float64 => 64,
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

/// Return the canonical hidden header types for one slice value.
pub fn slice_header_types(
    kind: ReferenceKind,
    element: TypeReference,
    access: Access,
    address_space: AddressSpace,
) -> (Type, Type) {
    let data = Type::Reference {
        kind,
        lifetime: Lifetime::empty(),
        address_space,
        access,
        pointee: element,
        is_nullable: false,
    };
    let length = Type::Usize;

    (data, length)
}

/// Return the signature reference carried by one callable type.
pub fn callable_signature(ty: &Type) -> Option<TypeReference> {
    match ty {
        Type::FunctionPointer { signature } | Type::Callable { signature } => Some(*signature),
        _ => None,
    }
}

/// Return the parameter and result types of one function signature.
pub fn function_signature_parts(ty: &Type) -> Option<(&[TypeReference], TypeReference)> {
    match ty {
        Type::FunctionSignature { parameters, result } => Some((parameters.as_slice(), *result)),
        _ => None,
    }
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
