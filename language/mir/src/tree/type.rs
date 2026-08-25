use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use destack_core::{FloatFormat, SectionEntry, StringId};

use crate::{
    Constant, Discriminant, Lifetime, LifetimeParameter, LocalNodeId, Node, NodeType,
    SignatureParameter, StaticId, StorageSet, Tree, TypeId,
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
    /// Return whether this access can write through the reference.
    pub fn can_write(self) -> bool {
        matches!(self, Access::Mutable | Access::Exclusive)
    }

    /// Return whether this access excludes overlapping borrows.
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
    /// Immutable link-written storage, reachable from every space.
    Constant,
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
            "constant" => Space::Constant,
            _ => return None,
        })
    }

    /// Return the canonical source name for this space.
    pub const fn label(self) -> &'static str {
        match self {
            Space::Local => "local",
            Space::Shared => "shared",
            Space::Constant => "constant",
        }
    }

    /// Return the backing memory space set.
    pub const fn space_set(self) -> StorageSet {
        match self {
            Space::Local => StorageSet::LOCAL,
            Space::Shared => StorageSet::SHARED,
            Space::Constant => StorageSet::GLOBAL,
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
    Default,
    Serialize,
    Deserialize,
    Reflect,
    SectionEntry,
)]
pub enum Storage {
    /// Worker-local heap storage.
    #[default]
    LocalHeap,
    /// Runtime-shared heap storage.
    SharedHeap,
    /// Frame storage inside the current activation.
    Frame,
    /// Immutable link-written constant storage.
    Constant,
    /// Worker-local static storage.
    LocalStatic,
    /// Runtime-shared static storage.
    SharedStatic,
}

impl Storage {
    /// Return the heap storage of one space.
    pub const fn heap(space: Space) -> Self {
        match space {
            Space::Local => Self::LocalHeap,
            Space::Shared => Self::SharedHeap,
            Space::Constant => Self::Constant,
        }
    }

    /// Return the static storage of one space.
    pub const fn global(space: Space) -> Self {
        match space {
            Space::Local => Self::LocalStatic,
            Space::Shared => Self::SharedStatic,
            Space::Constant => Self::Constant,
        }
    }

    /// Return the space coordinate of this storage.
    pub const fn space(self) -> Space {
        match self {
            Self::LocalHeap | Self::Frame | Self::LocalStatic => Space::Local,
            Self::SharedHeap | Self::SharedStatic => Space::Shared,
            Self::Constant => Space::Constant,
        }
    }

    /// Return the residence coordinate of this storage.
    pub const fn residence(self) -> Residence {
        match self {
            Self::Frame => Residence::Frame,
            Self::LocalHeap | Self::SharedHeap => Residence::Heap,
            Self::Constant | Self::LocalStatic | Self::SharedStatic => Residence::Static,
        }
    }

    /// Return the heap ownership domain when this is heap storage.
    pub const fn heap_space(self) -> Option<Space> {
        match self {
            Self::LocalHeap => Some(Space::Local),
            Self::SharedHeap => Some(Space::Shared),
            Self::Frame | Self::Constant | Self::LocalStatic | Self::SharedStatic => None,
        }
    }

    /// Return whether this storage is shared across workers.
    pub const fn is_shared(self) -> bool {
        matches!(self.space(), Space::Shared)
    }

    /// Return the canonical MIR text for this storage, the local space eliding.
    pub const fn label(self) -> &'static str {
        match self {
            Self::LocalHeap => "local",
            Self::SharedHeap => "shared",
            Self::Frame => "frame",
            Self::Constant => "constant",
            Self::LocalStatic => "static",
            Self::SharedStatic => "shared static",
        }
    }

    /// Return the symbol path segment for this storage.
    pub const fn segment(self) -> &'static str {
        match self {
            Self::LocalHeap => "local",
            Self::SharedHeap => "shared",
            Self::Frame => "frame",
            Self::Constant => "constant",
            Self::LocalStatic => "static",
            Self::SharedStatic => "sharedStatic",
        }
    }

    /// Return the storage region set used by memory effects.
    pub const fn storage_set(self) -> StorageSet {
        match self {
            Self::LocalHeap => StorageSet::LOCAL,
            Self::SharedHeap => StorageSet::SHARED,
            Self::Frame => StorageSet::FRAME,
            Self::Constant => StorageSet::GLOBAL,
            Self::LocalStatic => StorageSet::GLOBAL.union(StorageSet::LOCAL),
            Self::SharedStatic => StorageSet::GLOBAL.union(StorageSet::SHARED),
        }
    }
}

/// Storage residence axis: where the bytes live across activations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Residence {
    /// Inside the current activation frame.
    Frame,
    /// Allocated on a heap.
    Heap,
    /// Written once into static storage.
    Static,
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
}

impl ReferenceKind {
    /// Return the canonical MIR name.
    pub fn name(self) -> &'static str {
        match self {
            ReferenceKind::Managed => "managed",
            ReferenceKind::Unique => "unique",
            ReferenceKind::Borrowed => "borrowed",
        }
    }
}

/// Permitted invocation count for a callable value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum Multiplicity {
    /// The callable may be invoked any number of times.
    Repeatable,
    /// The callable may be invoked at most once.
    Once,
}

impl Multiplicity {
    /// Parse a canonical MIR name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "repeatable" => Some(Self::Repeatable),
            "once" => Some(Self::Once),
            _ => None,
        }
    }

    /// Return the canonical MIR name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Repeatable => "repeatable",
            Self::Once => "once",
        }
    }
}

/// Nullish values admitted by reference-like types.
#[repr(u8)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Reflect, SectionEntry,
)]
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
    /// Return whether this type can be copied freely.
    pub fn is_yes(self) -> bool {
        matches!(self, Copy::Yes)
    }

    /// Return whether this type is move only.
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
        /// The reference kind of the erased payload.
        kind: ReferenceKind,
        /// Lifetime roots for a borrowed erased payload.
        lifetime: Lifetime,
        /// The lowered dynamic constraint type.
        constraint: TypeId,
        /// The backing storage of the erased payload.
        storage: Storage,
        /// The access exposed through the erased payload.
        access: Access,
        /// The nullish values allowed by this dynamic descriptor.
        nullability: Nullability,
    },
    /// Reference with explicit kind and access.
    Reference {
        /// The reference kind.
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
    /// Process-local machine pointer.
    Pointer {
        /// The pointed-to type.
        pointee: TypeId,
        /// The access exposed through this pointer.
        access: Access,
        /// The nullish values allowed by this pointer.
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
    /// Nominal newtype over one wrapped type.
    Newtype {
        /// The wrapped type.
        inner: TypeId,
        /// Copy of this newtype.
        copy: Copy,
    },
    /// Sum value with one logical discriminant and case payloads.
    Variant {
        /// The logical discriminant type.
        discriminant: TypeId,
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
        /// The permitted number of invocations.
        multiplicity: Multiplicity,
        /// The reference kind of the captured environment.
        kind: ReferenceKind,
        /// Lifetime roots for a borrowed captured environment.
        lifetime: Lifetime,
        /// The backing storage of the captured environment.
        storage: Storage,
        /// The access exposed through the captured environment.
        access: Access,
        /// The nullish values allowed by this function descriptor.
        nullability: Nullability,
    },
    /// Function pointer type.
    FunctionPointer {
        /// The bare function signature.
        signature: TypeId,
    },

    /// Type use with applied lifetime arguments.
    Application {
        /// The type being applied.
        base: TypeId,
        /// The applied lifetime arguments.
        lifetimes: Vec<Lifetime>,
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

impl VariantCase {
    /// Return the logical discriminant bits when this case uses a scalar constant.
    pub fn discriminant(&self) -> Option<Discriminant> {
        let bits = match &self.discriminant {
            Constant::Int { value, width, .. } => (*value as u128) & Self::integer_mask(*width),
            Constant::UInt { value, width } => *value & Self::integer_mask(*width),
            Constant::Boolean { value } => *value as u128,
            _ => return None,
        };

        Some(Discriminant::from_bits(bits))
    }

    /// Return the low-bit mask for one integer width.
    const fn integer_mask(width: u16) -> u128 {
        if width >= u128::BITS as u16 {
            u128::MAX
        } else {
            (1u128 << width) - 1
        }
    }
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

    /// Parse one primitive MIR type name.
    pub fn from_primitive_name(name: &str) -> Option<Self> {
        Some(match name {
            "never" => Type::Never,
            "void" => Type::Void,
            "boolean" => Type::Boolean,
            "char" => Type::Character,
            name if let Some(width) = name.strip_prefix("int")
                && let Ok(width) = width.parse() =>
            {
                Type::Int {
                    width,
                    is_signed: true,
                }
            }
            name if let Some(width) = name.strip_prefix("uint")
                && let Ok(width) = width.parse() =>
            {
                Type::Int {
                    width,
                    is_signed: false,
                }
            }
            "isize" => Type::Isize,
            "usize" => Type::Usize,
            "float32" => Type::FLOAT32,
            "float64" => Type::FLOAT64,
            "typeDescriptor" => Type::TypeDescriptor,
            "typeId" => Type::TypeId,
            _ => return None,
        })
    }

    /// Return one structural field type.
    pub fn field_type(&self, index: u32, tree: &Tree) -> Option<TypeId> {
        match self {
            Type::Struct { fields, .. } => {
                fields.get(index as usize).map(|field| tree.get(*field).ty)
            }
            Type::Tuple { elements, .. } => elements.get(index as usize).copied(),
            Type::Newtype { inner, .. } if index == 0 => Some(*inner),
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

    /// Return whether this is a signed integer type.
    pub const fn integer_signedness(&self) -> Option<bool> {
        match self {
            Type::Int { is_signed, .. } => Some(*is_signed),
            Type::Isize => Some(true),
            Type::Usize => Some(false),
            _ => None,
        }
    }

    /// Return whether operators on this type use floating point semantics.
    pub fn is_float(&self, tree: &Tree) -> bool {
        match self {
            Type::Float(_) => true,
            Type::Vector { element, .. } => matches!(tree.get(*element), Type::Float(_)),
            _ => false,
        }
    }

    /// Return whether this type is a scalar.
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
                | Type::Pointer { .. }
                | Type::Vector { .. }
        )
    }

    /// Return the byte size when it follows directly from the type.
    pub fn byte_size(&self, tree: &Tree, pointer_width_bits: u16) -> Option<u64> {
        match self {
            Type::Int { width, .. } => Self::byte_width(*width),
            Type::Isize | Type::Usize | Type::Pointer { .. } => {
                Self::byte_width(pointer_width_bits)
            }
            Type::Float(format) => Self::byte_width(format.width()),
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
            Type::Application { base, .. } => tree.get(*base).byte_size(tree, pointer_width_bits),
            _ => None,
        }
    }

    /// Return whether this type is a process-local machine pointer.
    pub fn is_pointer(&self) -> bool {
        matches!(self, Type::Pointer { .. })
    }

    /// Return whether this type is a managed reference.
    pub fn is_managed_reference(&self) -> bool {
        self.reference_kind() == Some(ReferenceKind::Managed)
    }

    /// Return whether this type is a borrowed reference.
    pub fn is_borrowed_reference(&self) -> bool {
        self.reference_kind() == Some(ReferenceKind::Borrowed)
    }

    /// Return whether this type is a writable borrowed reference.
    pub fn is_writable_borrowed_reference(&self) -> bool {
        self.is_borrowed_reference()
            && self
                .reference_access()
                .is_some_and(|access| access.can_write())
    }

    /// Return whether this type is a unique reference.
    pub fn is_unique_reference(&self) -> bool {
        self.reference_kind() == Some(ReferenceKind::Unique)
    }

    /// Return whether this reference-like value may alias another reference.
    pub fn is_aliasable_reference(&self) -> bool {
        self.is_reference_representation()
            && !self.is_unique_reference()
            && !self.reference_access().is_some_and(Access::is_exclusive)
    }

    /// Return whether this type owns unique storage.
    pub fn is_unique_storage(&self) -> bool {
        self.reference_kind() == Some(ReferenceKind::Unique)
    }

    /// Return whether this type carries a reference as its intrinsic representation.
    pub fn is_reference_representation(&self) -> bool {
        matches!(
            self,
            Type::Dynamic { .. }
                | Type::Reference { .. }
                | Type::Slice { .. }
                | Type::Function { .. }
        )
    }

    /// Replace the lifetime of a reference-like value.
    pub fn set_lifetime(&mut self, replacement: Lifetime) {
        match self {
            Type::Dynamic { lifetime, .. }
            | Type::Reference { lifetime, .. }
            | Type::Slice { lifetime, .. }
            | Type::Function { lifetime, .. } => *lifetime = replacement,
            _ => {}
        }
    }

    /// Return this type with its verification-only lifetime erased.
    pub fn erased_lifetime(&self) -> Type {
        let mut erased = self.clone();
        erased.set_lifetime(Lifetime::empty());

        erased
    }

    /// Return the reference kind for reference-like values.
    pub fn reference_kind(&self) -> Option<ReferenceKind> {
        match self {
            Type::Dynamic { kind, .. }
            | Type::Reference { kind, .. }
            | Type::Slice { kind, .. }
            | Type::Function { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    /// Return the lifetime for reference-like values.
    pub fn reference_lifetime(&self) -> Option<&Lifetime> {
        match self {
            Type::Dynamic { lifetime, .. }
            | Type::Reference { lifetime, .. }
            | Type::Slice { lifetime, .. }
            | Type::Function { lifetime, .. } => Some(lifetime),
            _ => None,
        }
    }

    /// Return the access for reference-like values.
    pub fn reference_access(&self) -> Option<Access> {
        match self {
            Type::Dynamic { access, .. }
            | Type::Reference { access, .. }
            | Type::Slice { access, .. }
            | Type::Function { access, .. } => Some(*access),
            _ => None,
        }
    }

    /// Return the storage addressed by one reference-like value.
    pub fn reference_storage(&self) -> Option<Storage> {
        match self {
            Type::Dynamic { storage, .. }
            | Type::Reference { storage, .. }
            | Type::Slice { storage, .. }
            | Type::Function { storage, .. } => Some(*storage),
            _ => None,
        }
    }

    /// Return the nullish values accepted by one reference-like type.
    pub fn nullability(&self) -> Option<Nullability> {
        match self {
            Type::Dynamic { nullability, .. }
            | Type::Reference { nullability, .. }
            | Type::Slice { nullability, .. }
            | Type::Function { nullability, .. }
            | Type::Pointer { nullability, .. } => Some(*nullability),
            _ => None,
        }
    }

    /// Set the nullish values admitted by one reference-like type.
    pub fn set_nullability(&mut self, value: Nullability) -> bool {
        let nullability = match self {
            Type::Dynamic { nullability, .. }
            | Type::Reference { nullability, .. }
            | Type::Slice { nullability, .. }
            | Type::Function { nullability, .. }
            | Type::Pointer { nullability, .. } => nullability,
            _ => return false,
        };
        *nullability = value;

        true
    }

    /// Return the hidden storage types for one slice value.
    pub fn slice(
        kind: ReferenceKind,
        element: TypeId,
        access: Access,
        storage: Storage,
    ) -> (Type, Type) {
        let reference = Type::Reference {
            kind,
            lifetime: Lifetime::empty(),
            storage,
            access,
            pointee: element,
            nullability: Nullability::None,
        };
        let length = Type::Usize;

        (reference, length)
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
    pub fn copy(&self, tree: &Tree) -> Copy {
        match self {
            // parse recovery nodes carry no copyable value
            Type::Error => Copy::No,

            // the uninhabited type has no values to move
            Type::Never => Copy::Yes,
            // primitives are always trivially copyable
            Type::Void
            | Type::Boolean
            | Type::Character
            | Type::Int { .. }
            | Type::Isize
            | Type::Usize
            | Type::Float { .. }
            | Type::TypeDescriptor
            | Type::TypeId
            | Type::Pointer { .. } => Copy::Yes,

            // atomic cells are storage, so each use consumes them
            Type::Atomic { .. } => Copy::No,

            // initialization tokens are linear capabilities
            Type::Uninit { .. } | Type::ManuallyDrop { .. } => Copy::No,

            // once functions are consumed by invocation
            Type::Function {
                multiplicity: Multiplicity::Once,
                ..
            } => Copy::No,

            // unique reference-like values carry ownership of their backing storage
            Type::Dynamic { kind, .. }
            | Type::Reference { kind, .. }
            | Type::Slice { kind, .. }
            | Type::Function { kind, .. } => match kind {
                ReferenceKind::Unique => Copy::No,
                ReferenceKind::Managed | ReferenceKind::Borrowed => Copy::Yes,
            },

            // aggregates have explicit copy
            Type::FixedArray { copy, .. }
            | Type::Tuple { copy, .. }
            | Type::Struct { copy, .. }
            | Type::Newtype { copy, .. }
            | Type::Variant { copy, .. }
            | Type::Vector { copy, .. } => *copy,

            // signatures and thin function pointers contain no captured storage
            Type::FunctionSignature { .. } | Type::FunctionPointer { .. } => Copy::Yes,

            // lifetime application preserves the represented type's copy property
            Type::Application { base, .. } => tree.get(*base).copy(tree),
        }
    }

    /// Convert one bit width to bytes when byte aligned.
    fn byte_width(width: u16) -> Option<u64> {
        width.is_multiple_of(8).then_some(u64::from(width / 8))
    }
}

/// A concrete MIR floating-point type.
#[repr(u32)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub enum FloatType {
    /// A 32-bit IEEE-754 float.
    Float32,
    /// A 64-bit IEEE-754 float.
    Float64,
}

impl FloatType {
    /// Return the bit width.
    pub const fn width(self) -> u16 {
        match self {
            FloatType::Float32 => 32,
            FloatType::Float64 => 64,
        }
    }

    /// Return the canonical source label.
    pub fn label(self) -> &'static str {
        match self {
            FloatType::Float32 => "float32",
            FloatType::Float64 => "float64",
        }
    }

    /// Return the core representation format.
    pub fn format(self) -> FloatFormat {
        match self {
            FloatType::Float32 => FloatFormat::Float32,
            FloatType::Float64 => FloatFormat::Float64,
        }
    }
}

/// A field in a struct type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Field {
    /// The optional field name.
    pub name: Option<StringId>,
    /// Type of the field.
    pub ty: TypeId,
}

impl Node for Field {
    const TYPE: NodeType = NodeType::Field;
}

/// A named MIR type declaration.
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
    /// The directly inherited and implemented types.
    pub heritage: TypeHeritage,
}

impl Node for TypeDeclaration {
    const TYPE: NodeType = NodeType::TypeDeclaration;
}

/// Direct heritage for one nominal type.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct TypeHeritage {
    /// Directly inherited types.
    pub extends: Vec<TypeId>,
    /// Directly implemented interfaces.
    pub implements: Vec<TypeId>,
}

impl TypeHeritage {
    /// Return whether the type declares no direct supertypes.
    pub fn is_empty(&self) -> bool {
        self.extends.is_empty() && self.implements.is_empty()
    }
}

impl Type {
    /// Apply one mapping to every direct child type id of this type.
    pub fn map_child_type_ids(&mut self, map: &mut impl FnMut(TypeId) -> TypeId) {
        match self {
            Type::Reference { pointee, .. } | Type::Pointer { pointee, .. } => {
                *pointee = map(*pointee);
            }
            Type::Atomic { value } | Type::Uninit { value } | Type::ManuallyDrop { value } => {
                *value = map(*value);
            }
            Type::Dynamic { constraint, .. } => {
                *constraint = map(*constraint);
            }
            Type::FixedArray { element, .. }
            | Type::Slice { element, .. }
            | Type::Vector { element, .. } => {
                *element = map(*element);
            }
            Type::Tuple { elements, copy: _ } => {
                for element in elements {
                    *element = map(*element);
                }
            }
            Type::Newtype { inner, .. } => {
                *inner = map(*inner);
            }
            Type::Variant {
                discriminant,
                cases,
                copy: _,
            } => {
                *discriminant = map(*discriminant);
                for case in cases {
                    case.ty = map(case.ty);
                }
            }
            Type::FunctionSignature {
                parameters, result, ..
            } => {
                for parameter in parameters {
                    parameter.ty = map(parameter.ty);
                }
                *result = map(*result);
            }
            Type::FunctionPointer { signature } | Type::Function { signature, .. } => {
                *signature = map(*signature);
            }
            Type::Application { base, .. } => {
                *base = map(*base);
            }
            // struct children are field nodes, paired by their consumers
            Type::Struct { .. } => {}
            Type::Error
            | Type::Never
            | Type::Void
            | Type::Boolean
            | Type::Character
            | Type::Int { .. }
            | Type::Isize
            | Type::Usize
            | Type::Float { .. }
            | Type::TypeDescriptor
            | Type::TypeId => {}
        }
    }
}

/// One nullish constant a type can store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Nullish {
    /// The null constant.
    Null,
    /// The undefined constant.
    Undefined,
}

/// One position where a type stores an absent nullish constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NullishCase {
    /// The variant case holding the void payload.
    Case(u32),
    /// The nullish niche of a reference-family type.
    Niche,
}

impl Nullability {
    /// Return whether this niche admits one nullish constant.
    pub fn admits(self, nullish: Nullish) -> bool {
        matches!(
            (self, nullish),
            (
                Nullability::Null | Nullability::NullOrUndefined,
                Nullish::Null
            ) | (
                Nullability::Undefined | Nullability::NullOrUndefined,
                Nullish::Undefined
            )
        )
    }
}

impl Tree {
    /// Return where one type stores one nullish constant, when it does.
    pub fn nullish_case(&self, ty: TypeId, nullish: Nullish) -> Option<NullishCase> {
        match self.get(ty) {
            // read through lifetime applications onto the wrapped base
            Type::Application { base, .. } => self.nullish_case(*base, nullish),

            // variants store undefined as their void case
            Type::Variant { cases, .. } => match nullish {
                Nullish::Undefined => cases
                    .iter()
                    .position(|case| matches!(self.get(case.ty), Type::Void))
                    .map(|index| NullishCase::Case(index as u32)),
                Nullish::Null => None,
            },

            // read the nullish niche of reference-family types
            other => other
                .nullability()
                .filter(|nullability| nullability.admits(nullish))
                .map(|_| NullishCase::Niche),
        }
    }

    /// Return where one type stores undefined, when it does.
    pub fn undefined_case(&self, ty: TypeId) -> Option<NullishCase> {
        self.nullish_case(ty, Nullish::Undefined)
    }
}
