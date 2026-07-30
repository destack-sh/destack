use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_core::{FloatFormat, SectionEntry, StringId};

use crate::{
    Constant, Lifetime, LifetimeParameter, LocalNodeId, Node, NodeType, SignatureParameter,
    StaticId, StorageSet, Tree, TypeId,
};

/// Mutability of a storage binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Mutability {
    /// Immutable (const).
    Immutable,
    /// Mutable (var).
    Mutable,
}

/// Access exposed by a reference-like value.
#[repr(u32)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
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

/// Runtime ownership domain.
#[repr(u32)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub enum Space {
    /// Worker-local storage.
    #[default]
    Local,
    /// Runtime-shared storage.
    Shared,
}

impl Space {
    /// Return whether this is worker-local runtime storage.
    pub fn is_local(&self) -> bool {
        matches!(self, Space::Local)
    }

    /// Parse a canonical space name.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "local" => Space::Local,
            "shared" => Space::Shared,
            _ => return None,
        })
    }

    /// Return the canonical source name for this space.
    pub const fn label(self) -> &'static str {
        match self {
            Space::Local => "local",
            Space::Shared => "shared",
        }
    }

    /// Return the backing memory space set.
    pub const fn space_set(self) -> StorageSet {
        match self {
            Space::Local => StorageSet::LOCAL,
            Space::Shared => StorageSet::SHARED,
        }
    }
}

/// Static storage selected by one global declaration.
#[repr(u8)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Default,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub enum GlobalStorage {
    /// Immutable Program constant storage.
    Constant,
    /// Worker-local static storage.
    #[default]
    Local,
    /// Runtime-shared static storage.
    Shared,
}

impl GlobalStorage {
    /// Return the global storage region used by memory effects.
    pub const fn storage_set(self) -> StorageSet {
        match self {
            Self::Constant => StorageSet::GLOBAL,
            Self::Local => StorageSet::GLOBAL.union(StorageSet::LOCAL),
            Self::Shared => StorageSet::GLOBAL.union(StorageSet::SHARED),
        }
    }
}

/// Backing storage addressed by one reference-like value.
#[repr(u8)]
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub enum Storage {
    /// Heap storage in one ownership domain.
    Heap(Space),
    /// Frame storage inside the current activation.
    Frame,
    /// Static storage selected by one global declaration.
    Global(GlobalStorage),
}

impl Default for Storage {
    fn default() -> Self {
        Self::Heap(Space::Local)
    }
}

impl Storage {
    /// Return the heap ownership domain when this is heap storage.
    pub const fn heap_space(self) -> Option<Space> {
        match self {
            Self::Heap(space) => Some(space),
            Self::Frame | Self::Global(_) => None,
        }
    }

    /// Return the canonical MIR name for this storage.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Heap(space) => space.label(),
            Self::Frame => "frame",
            Self::Global(GlobalStorage::Constant) => "constant",
            Self::Global(GlobalStorage::Local) => "global",
            Self::Global(GlobalStorage::Shared) => "sharedGlobal",
        }
    }

    /// Return the storage region set used by memory effects.
    pub const fn storage_set(self) -> StorageSet {
        match self {
            Self::Heap(space) => space.space_set(),
            Self::Frame => StorageSet::FRAME,
            Self::Global(storage) => storage.storage_set(),
        }
    }
}

/// Kind of reference in MIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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

impl ReferenceKind {
    /// Return the canonical MIR name.
    pub fn name(self) -> &'static str {
        match self {
            ReferenceKind::Managed => "managed",
            ReferenceKind::Unique => "unique",
            ReferenceKind::Borrowed => "borrowed",
            ReferenceKind::Raw => "raw",
        }
    }
}

/// Invalid-value niches carried by pointer-like values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect)]
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

/// Copyability of a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Copy {
    /// Value can be copied freely.
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
#[repr(u32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum TensorDimensionOrder {
    /// Last dimension is contiguous.
    RowMajor,
    /// First dimension is contiguous.
    ColumnMajor,
}

/// Format for an owning tensor value.
#[repr(C, u32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum TensorFormat {
    /// Dense contiguous format.
    Dense {
        /// The dimension order.
        order: TensorDimensionOrder,
    },
}

impl TensorFormat {
    /// Return the default dense row-major tensor format.
    pub fn dense_row_major() -> Self {
        TensorFormat::Dense {
            order: TensorDimensionOrder::RowMajor,
        }
    }
}

/// Format descriptor for a tensor view.
#[repr(C, u32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum TensorViewFormat {
    /// Dense contiguous view.
    Dense {
        /// The dimension order.
        order: TensorDimensionOrder,
    },
    /// Explicit strided view.
    Strided,
}

impl TensorViewFormat {
    /// Return the default dense row-major tensor view format.
    pub fn dense_row_major() -> Self {
        TensorViewFormat::Dense {
            order: TensorDimensionOrder::RowMajor,
        }
    }
}

/// Placement descriptor for tensor storage.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum TensorSharding {
    /// Tensor storage is not partitioned across a mesh.
    Unsharded,
    /// Tensor storage is mapped across a mesh axis by axis.
    Sharding {
        /// The per-axis placement descriptors.
        axes: Vec<TensorShardingAxis>,
    },
}

impl TensorSharding {
    /// Return the default unsharded tensor placement.
    pub fn unsharded() -> Self {
        Self::Unsharded
    }
}

/// Per-axis placement descriptor for a sharded tensor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum TensorShardingAxis {
    /// Split one tensor axis across one mesh axis.
    Shard {
        /// The tensor axis being split.
        axis: i32,
    },
    /// Replicate values across one mesh axis.
    Replicate,
    /// Store partial results across one mesh axis.
    Partial {
        /// The reduction used to combine partial values.
        reduction: TensorReduction,
    },
}

/// Reduction used when partial tensor shards are combined.
#[repr(u32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum TensorReduction {
    /// Add partial values.
    Add,
    /// Multiply partial values.
    Multiply,
    /// Keep the minimum partial value.
    Minimum,
    /// Keep the maximum partial value.
    Maximum,
    /// Combine partial boolean values with AND.
    And,
    /// Combine partial boolean values with OR.
    Or,
}

/// Dimension size for tensor shapes and formats.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
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

/// Stable canonical fingerprint of one MIR type.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, Reflect,
)]
pub struct TypeFingerprint(u128);

impl TypeFingerprint {
    /// Restore one fingerprint from its persistent bits.
    pub const fn from_raw(raw: u128) -> Self {
        Self(raw)
    }

    /// Return the persistent fingerprint bits.
    pub const fn raw(self) -> u128 {
        self.0
    }
}

/// One logical MIR type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Type {
    /// Invalid type produced while recovering malformed MIR text.
    Error,
    /// Uninhabited type for execution that cannot complete normally.
    Never,
    /// Void / unit type (no value).
    Void,
    /// Boolean (1 bit logical, typically 1 byte).
    Boolean,
    /// Unicode scalar value.
    Character,
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
    /// Compact 32-bit runtime type identity token.
    TypeId,

    /// Atomic storage cell for one value type.
    Atomic {
        /// The stored value type.
        value: TypeId,
    },
    /// Runtime-erased value satisfying one dynamic constraint.
    Dynamic {
        /// The lowered dynamic constraint type.
        constraint: TypeId,
        /// The nullish values allowed by this erased value.
        nullability: Nullability,
        /// The space of the erased managed payload.
        space: Space,
    },
    /// Type use with applied lifetime arguments.
    WithLifetimes {
        /// The type being applied.
        base: TypeId,
        /// The applied lifetime arguments.
        lifetimes: Vec<Lifetime>,
    },
    /// Reference with explicit kind and access.
    Reference {
        /// The reference kind (managed, unique, borrowed, raw).
        kind: ReferenceKind,
        /// Lifetime roots for borrowed references.
        lifetime: Lifetime,
        /// The backing storage for this reference.
        storage: Storage,
        /// The access exposed through this reference.
        access: Access,
        /// The referenced type.
        pointee: TypeId,
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
        element: TypeId,
        /// The backing storage of the slice base.
        storage: Storage,
        /// The element access exposed by the slice.
        access: Access,
        /// The nullish values allowed by this slice descriptor.
        nullability: Nullability,
    },
    /// Linear token for one possibly uninitialized storage.
    Uninit {
        /// The value under construction.
        value: TypeId,
    },
    /// Owned storage whose automatic drop is suppressed.
    ManuallyDrop {
        /// The wrapped value.
        value: TypeId,
    },

    /// Fixed array: `[T; N]`.
    FixedArray {
        /// The element type of the array.
        element: TypeId,
        /// The number of elements in the array.
        length: u64,
        /// Copy of this fixed array type.
        copy: Copy,
    },
    /// Tuple: `(T1, T2, ...)`.
    Tuple {
        /// The element types of the tuple.
        elements: Vec<TypeId>,
        /// Copy of this tuple type.
        copy: Copy,
    },
    /// Struct.
    Struct {
        /// The fields of the struct.
        fields: Vec<LocalNodeId<Field>>,
        /// Copy of this struct type.
        copy: Copy,
    },
    /// Nominal newtype wrapping an inner type.
    Newtype {
        /// The wrapped inner type.
        inner: TypeId,
        /// Copy of this newtype.
        copy: Copy,
    },
    /// Sum value with one logical discriminant and payload storage.
    Variant {
        /// The logical discriminant type.
        discriminant: TypeId,
        /// The logical payload storage type.
        storage: TypeId,
        /// The cases keyed by discriminant value.
        cases: Vec<VariantCase>,
        /// Copy of this variant type.
        copy: Copy,
    },

    /// Fixed-width vector value.
    Vector {
        /// The element type.
        element: TypeId,
        /// The number of lanes.
        lanes: u32,
        /// Copy of this vector type.
        copy: Copy,
    },
    /// Ranked tensor value with static or dynamic shape.
    Tensor {
        /// The element type.
        element: TypeId,
        /// The tensor allocation space.
        space: Space,
        /// The static shape.
        shape: Vec<TensorDimension>,
        /// The tensor format.
        format: TensorFormat,
        /// The tensor placement.
        sharding: TensorSharding,
        /// Copy of this tensor type.
        copy: Copy,
    },
    /// Reference-like view into tensor-shaped memory.
    TensorView {
        /// The reference kind (managed, unique, borrowed, raw).
        kind: ReferenceKind,
        /// Lifetime roots for borrowed tensor views.
        lifetime: Lifetime,
        /// The backing storage for this view.
        storage: Storage,
        /// The access exposed through this view.
        access: Access,
        /// The element type.
        element: TypeId,
        /// The static shape.
        shape: Vec<TensorDimension>,
        /// The tensor view format.
        format: TensorViewFormat,
        /// The tensor placement.
        sharding: TensorSharding,
        /// The nullish values allowed by this view descriptor.
        nullability: Nullability,
    },

    /// Bare function signature.
    FunctionSignature {
        /// Lifetime parameters in signature-local slot order.
        lifetimes: Vec<LifetimeParameter>,
        /// The parameters of the function.
        parameters: Vec<SignatureParameter>,
        /// The result type of the function.
        result: TypeId,
    },
    /// Function value type.
    Function {
        /// The bare function signature.
        signature: TypeId,
        /// The captured environment reference.
        environment: TypeId,
    },
    /// Function pointer type.
    FunctionPointer {
        /// The bare function signature.
        signature: TypeId,
    },

    /// Move-only handle for one coroutine execution.
    Continuation {
        /// The value accepted when resuming after a yield.
        resume_type: TypeId,
        /// The value produced by each yield.
        yield_type: TypeId,
        /// The value produced by final return.
        return_type: TypeId,
    },
    /// Move-only capability for settling one parked asynchronous continuation.
    Waiter {
        /// The value accepted when queueing the suspended execution.
        value_type: TypeId,
    },
}

/// One sum case.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct VariantCase {
    /// The discriminant constant selecting this case.
    pub discriminant: Constant,
    /// The logical payload type.
    pub ty: TypeId,
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

    pub const FLOAT16: Type = Type::Float(FloatType::Float16);
    pub const BFLOAT16: Type = Type::Float(FloatType::Bfloat16);
    pub const FLOAT32: Type = Type::Float(FloatType::Float32);
    pub const FLOAT64: Type = Type::Float(FloatType::Float64);

    /// Return one structural field type.
    pub fn field_type(&self, index: u32, tree: &Tree) -> Option<TypeId> {
        match self {
            Type::Struct { fields, .. } => {
                fields.get(index as usize).map(|field| tree.get(*field).ty)
            }
            Type::Tuple { elements, .. } => elements.get(index as usize).copied(),
            Type::Newtype { inner, .. } if index == 0 => Some(*inner),
            Type::Variant {
                discriminant,
                storage,
                ..
            } => match index {
                0 => Some(*discriminant),
                1 => Some(*storage),
                _ => None,
            },
            _ => None,
        }
    }

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
                | Type::Character
                | Type::Int { .. }
                | Type::Isize
                | Type::Usize
                | Type::Float(_)
                | Type::TypeDescriptor
                | Type::TypeId
                | Type::Reference { .. }
                | Type::Vector { .. }
                | Type::Continuation { .. }
                | Type::Waiter { .. }
        )
    }

    /// Return the byte size when it follows directly from the type.
    pub fn byte_size(&self, tree: &Tree, pointer_width_bits: u16) -> Option<u64> {
        match self {
            Type::Int { width, .. } => byte_width(*width),
            Type::Isize | Type::Usize => byte_width(pointer_width_bits),
            Type::Continuation { .. } | Type::Waiter { .. } => byte_width(u64::BITS as u16),
            Type::Float(format) => byte_width(format.width()),
            Type::WithLifetimes { base, .. } => tree.get(*base).byte_size(tree, pointer_width_bits),
            Type::Uninit { value } | Type::ManuallyDrop { value } => {
                tree.get(*value).byte_size(tree, pointer_width_bits)
            }
            Type::Newtype { inner, .. } => tree.get(*inner).byte_size(tree, pointer_width_bits),
            Type::FixedArray {
                element, length, ..
            } => {
                let element_size = tree.get(*element).byte_size(tree, pointer_width_bits)?;

                element_size.checked_mul(*length)
            }
            _ => None,
        }
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

    /// Whether this type owns unique storage.
    pub fn is_unique_storage(&self) -> bool {
        matches!(
            self,
            Type::Reference {
                kind: ReferenceKind::Unique,
                ..
            } | Type::Slice {
                kind: ReferenceKind::Unique,
                ..
            } | Type::TensorView {
                kind: ReferenceKind::Unique,
                ..
            }
        )
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

    /// Return the nullish values accepted by one reference-like type.
    pub fn nullability(&self) -> Option<Nullability> {
        match self {
            Type::Reference { nullability, .. }
            | Type::Slice { nullability, .. }
            | Type::TensorView { nullability, .. } => Some(*nullability),
            _ => None,
        }
    }

    /// Return the hidden storage types for one slice value.
    pub fn slice(
        kind: ReferenceKind,
        element: TypeId,
        access: Access,
        storage: Storage,
    ) -> (Type, Type) {
        let data = Type::Reference {
            kind,
            lifetime: Lifetime::empty(),
            storage,
            access,
            pointee: element,
            nullability: Nullability::None,
        };
        let length = Type::Usize;

        (data, length)
    }

    /// Return the signature reference carried by this callable type.
    pub fn callable_signature(&self) -> Option<TypeId> {
        match self {
            Type::FunctionPointer { signature } | Type::Function { signature, .. } => {
                Some(*signature)
            }
            _ => None,
        }
    }

    /// Return the lifetimes, parameters, and result type of this function signature.
    pub fn function_signature_parts(
        &self,
    ) -> Option<(&[LifetimeParameter], &[SignatureParameter], TypeId)> {
        match self {
            Type::FunctionSignature {
                lifetimes,
                parameters,
                result,
                ..
            } => Some((lifetimes.as_slice(), parameters.as_slice(), *result)),
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
    pub fn copy(&self, tree: &Tree) -> Copy {
        match self {
            // parse recovery nodes are never copyable semantic values
            Type::Error => Copy::No,

            // the uninhabited type has no values to move
            Type::Never => Copy::Yes,
            // lifetime application preserves the represented type's copy property
            Type::WithLifetimes { base, .. } => tree.get(*base).copy(tree),

            // primitives are always trivially copyable
            Type::Void
            | Type::Boolean
            | Type::Character
            | Type::Int { .. }
            | Type::Isize
            | Type::Usize
            | Type::Float { .. }
            | Type::TypeDescriptor
            | Type::TypeId => Copy::Yes,

            // atomic cells are storage, not freely copied values
            Type::Atomic { .. } => Copy::No,

            // dynamic values carry one managed payload reference and witness table
            Type::Dynamic { .. } => Copy::Yes,

            // initialization tokens are linear capabilities
            Type::Uninit { .. } | Type::ManuallyDrop { .. } => Copy::No,

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
            Type::FixedArray { copy, .. }
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

            // function values copy the handle, not the environment payload
            Type::FunctionSignature { .. }
            | Type::FunctionPointer { .. }
            | Type::Function { .. } => Copy::Yes,

            // execution handles uniquely own suspended execution
            Type::Continuation { .. } | Type::Waiter { .. } => Copy::No,
        }
    }
}

/// Convert one bit width to bytes when byte aligned.
fn byte_width(width: u16) -> Option<u64> {
    width.is_multiple_of(8).then_some(u64::from(width / 8))
}

/// A concrete MIR floating-point type.
#[repr(u32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum FloatType {
    /// A 16-bit IEEE-754 binary16 float.
    Float16,
    /// A 16-bit bfloat format.
    Bfloat16,
    /// A 32-bit IEEE-754 float.
    Float32,
    /// A 64-bit IEEE-754 float.
    Float64,
}

impl FloatType {
    /// Return the bit width.
    pub fn width(self) -> u16 {
        match self {
            FloatType::Float16 | FloatType::Bfloat16 => 16,
            FloatType::Float32 => 32,
            FloatType::Float64 => 64,
        }
    }

    /// Return the canonical source label.
    pub fn label(self) -> &'static str {
        match self {
            FloatType::Float16 => "float16",
            FloatType::Bfloat16 => "bfloat16",
            FloatType::Float32 => "float32",
            FloatType::Float64 => "float64",
        }
    }

    /// Return the core representation format.
    pub fn format(self) -> FloatFormat {
        match self {
            FloatType::Float16 => FloatFormat::Float16,
            FloatType::Bfloat16 => FloatFormat::Bfloat16,
            FloatType::Float32 => FloatFormat::Float32,
            FloatType::Float64 => FloatFormat::Float64,
        }
    }
}

/// A field in a struct type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Field {
    /// Name (optional).
    pub name: Option<StringId>,
    /// Type of the field.
    pub ty: TypeId,
}

impl Node for Field {
    const TYPE: NodeType = NodeType::Field;
}

/// A named type declaration in MIR text format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TypeDeclaration {
    /// The declaration name.
    pub name: StringId,
    /// Concrete generic arguments specializing this type.
    pub arguments: Vec<StaticId>,
    /// Lifetime parameters in type-local slot order.
    pub lifetimes: Vec<LifetimeParameter>,
    /// The identified type.
    pub ty: TypeId,
}

impl Node for TypeDeclaration {
    const TYPE: NodeType = NodeType::TypeDeclaration;
}
