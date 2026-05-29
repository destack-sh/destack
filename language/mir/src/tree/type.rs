use serde::{Deserialize, Serialize};

use destack_core::StringId;

use crate::{BorrowObligation, Constant, Lifetime, LocalNodeId, Node, NodeType, TypeReference};

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
    /// Mutable access.
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

/// Space for a reference.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Space {
    /// Local runtime storage.
    #[default]
    Local,
    /// Shared runtime storage.
    Shared,
    /// Frame-slot storage inside one activation.
    Frame,
    /// Static memory.
    Static,
}

impl Space {
    /// Check if this is the local runtime storage space.
    pub fn is_local(&self) -> bool {
        matches!(self, Space::Local)
    }

    /// Parse a canonical space name.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "local" => Space::Local,
            "shared" => Space::Shared,
            "frame" => Space::Frame,
            "static" => Space::Static,
            _ => return None,
        })
    }

    /// Return the canonical source name for this space.
    pub fn label(&self) -> &str {
        match self {
            Space::Local => "local",
            Space::Shared => "shared",
            Space::Frame => "frame",
            Space::Static => "static",
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

/// Invalid-value niches carried by pointer-like values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Nullability {
    /// No nullish values are allowed.
    #[default]
    None,
    /// `null` is allowed.
    Null,
    /// `undefined` is allowed.
    Undefined,
    /// `null` and `undefined` are allowed.
    NullOrUndefined,
}

impl Nullability {
    /// Return whether null is a valid value.
    #[inline]
    pub const fn allows_null(self) -> bool {
        matches!(self, Nullability::Null | Nullability::NullOrUndefined)
    }

    /// Return whether undefined is a valid value.
    #[inline]
    pub const fn allows_undefined(self) -> bool {
        matches!(self, Nullability::Undefined | Nullability::NullOrUndefined)
    }

    /// Return whether any nullish sentinel is valid.
    #[inline]
    pub const fn allows_nullish(self) -> bool {
        !matches!(self, Nullability::None)
    }
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

/// Dimension order for dense tensor storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TensorDimensionOrder {
    /// Last dimension is contiguous.
    RowMajor,
    /// First dimension is contiguous.
    ColumnMajor,
}

/// Layout for an owning tensor value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TensorLayout {
    /// Dense contiguous layout.
    Dense {
        /// The dimension order.
        order: TensorDimensionOrder,
    },
}

impl TensorLayout {
    /// Return the default dense row-major tensor layout.
    pub fn dense_row_major() -> Self {
        TensorLayout::Dense {
            order: TensorDimensionOrder::RowMajor,
        }
    }
}

/// Layout descriptor for a tensor view.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TensorViewLayout {
    /// Dense contiguous view.
    Dense {
        /// The dimension order.
        order: TensorDimensionOrder,
    },
    /// Explicit strided view.
    Strided,
}

impl TensorViewLayout {
    /// Return the default dense row-major tensor view layout.
    pub fn dense_row_major() -> Self {
        TensorViewLayout::Dense {
            order: TensorDimensionOrder::RowMajor,
        }
    }
}

/// Dimension size for tensor shapes and layouts.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TensorDimension {
    /// Compile time static dimension size.
    Static(u64),
    /// Symbolic runtime dimension shared across tensors.
    Symbol(String),
    /// Runtime dynamic dimension size.
    Dynamic,
}

impl TensorDimension {
    /// Check whether this dimension is dynamic.
    pub fn is_dynamic(&self) -> bool {
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
        /// The space for this reference.
        space: Space,
        /// The access exposed through this reference.
        access: Access,
        /// The referenced type.
        pointee: TypeReference,
        /// The nullish values allowed by this reference.
        nullability: Nullability,
    },
    /// Slice into memory, a repeated element with explicit kind and access.
    Slice {
        /// The reference kind of the slice base.
        kind: ReferenceKind,
        /// Lifetime roots for borrowed slices.
        lifetime: Lifetime,
        /// The element type of the slice.
        element: TypeReference,
        /// The space of the slice base.
        space: Space,
        /// The element access exposed by the slice.
        access: Access,
        /// The nullish values allowed by this slice descriptor.
        nullability: Nullability,
    },
    /// Linear token for one uninitialized allocation.
    Uninit {
        /// The value under construction.
        value: TypeReference,
    },

    /// Fixed-size array: `[T; N]`.
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
    /// Physical tagged sum value.
    Variant {
        /// The tag value type.
        tag: TypeReference,
        /// The physical payload storage type.
        storage: TypeReference,
        /// The cases keyed by tag value.
        cases: Vec<VariantCase>,
        /// Copy of this variant type.
        copy: Copy,
    },

    /// Fixed-width vector value.
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
        /// The space for this view.
        space: Space,
        /// The access exposed through this view.
        access: Access,
        /// The element type.
        element: TypeReference,
        /// The static shape.
        shape: Vec<TensorDimension>,
        /// The tensor view layout.
        layout: TensorViewLayout,
        /// The nullish values allowed by this view descriptor.
        nullability: Nullability,
    },

    /// Bare function signature.
    FunctionSignature {
        /// The parameters of the function.
        parameters: Vec<TypeReference>,
        /// The result type of the function.
        result: TypeReference,
        /// Borrow obligations callers must satisfy.
        borrow_obligations: Vec<BorrowObligation>,
    },
    /// Function pointer type.
    FunctionPointer {
        /// The bare function signature.
        signature: TypeReference,
    },
    /// Closure value with code and environment.
    Closure {
        /// The bare function signature.
        signature: TypeReference,
        /// The closure environment reference.
        environment: TypeReference,
    },
}

/// One physical tagged sum case.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VariantCase {
    /// The tag constant selecting this case.
    pub tag: Constant,
    /// The logical payload type.
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
        self.reference_kind() == Some(ReferenceKind::Raw)
    }

    /// Whether this type is a managed reference.
    pub fn is_managed_reference(&self) -> bool {
        self.reference_kind() == Some(ReferenceKind::Managed)
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
        self.reference_kind() == Some(ReferenceKind::Borrowed)
    }

    /// Whether this type is a writable borrowed reference.
    pub fn is_writable_borrowed_reference(&self) -> bool {
        self.is_borrowed_reference()
            && self
                .reference_access()
                .is_some_and(|access| access.can_write())
    }

    /// Whether this type is a unique reference.
    pub fn is_unique_reference(&self) -> bool {
        self.reference_kind() == Some(ReferenceKind::Unique)
    }

    /// Return the reference kind for reference-like values.
    pub fn reference_kind(&self) -> Option<ReferenceKind> {
        match self {
            Type::Reference { kind, .. }
            | Type::Slice { kind, .. }
            | Type::TensorView { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    /// Return the lifetime for reference-like values.
    pub fn reference_lifetime(&self) -> Option<&Lifetime> {
        match self {
            Type::Reference { lifetime, .. }
            | Type::Slice { lifetime, .. }
            | Type::TensorView { lifetime, .. } => Some(lifetime),
            _ => None,
        }
    }

    /// Return the access for reference-like values.
    pub fn reference_access(&self) -> Option<Access> {
        match self {
            Type::Reference { access, .. }
            | Type::Slice { access, .. }
            | Type::TensorView { access, .. } => Some(*access),
            _ => None,
        }
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

            // initialization tokens are linear capabilities
            Type::Uninit { .. } => Copy::No,

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
            | Type::Variant { copy, .. }
            | Type::Vector { copy, .. }
            | Type::Tensor { copy, .. } => *copy,

            // unique tensor views carry ownership of typed heap storage
            Type::TensorView { kind, .. } => match kind {
                ReferenceKind::Unique => Copy::No,
                ReferenceKind::Managed | ReferenceKind::Borrowed | ReferenceKind::Raw => Copy::Yes,
            },

            // closure values copy the handle, not the environment payload
            Type::FunctionSignature { .. }
            | Type::FunctionPointer { .. }
            | Type::Closure { .. } => Copy::Yes,
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
    space: Space,
) -> (Type, Type) {
    let data = Type::Reference {
        kind,
        lifetime: Lifetime::empty(),
        space,
        access,
        pointee: element,
        nullability: Nullability::None,
    };
    let length = Type::Usize;

    (data, length)
}

/// Return the signature reference carried by one callable type.
pub fn callable_signature(ty: &Type) -> Option<TypeReference> {
    match ty {
        Type::FunctionPointer { signature } | Type::Closure { signature, .. } => Some(*signature),
        _ => None,
    }
}

/// Return the parameter and result types of one function signature.
pub fn function_signature_parts(ty: &Type) -> Option<(&[TypeReference], TypeReference)> {
    match ty {
        Type::FunctionSignature {
            parameters, result, ..
        } => Some((parameters.as_slice(), *result)),
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
