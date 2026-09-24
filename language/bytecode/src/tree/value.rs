use destack_core::SectionEntry;
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::{Scalar, TypeId, VectorType};

/// The type and register word width of one bytecode value.
#[repr(C)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct ValueType {
    /// The value representation category.
    tag: ValueTag,
    /// The scalar representation for scalar and vector values.
    scalar: u8,
    /// The ownership and memory space for reference values.
    reference: ReferenceType,
    /// The contiguous register words occupied by the value.
    word_count: u16,
    /// The lane count for vector values.
    lane_count: u16,
    /// The object-local type selected by the tag.
    type_id: u32,
}

/// The ownership carried by one bytecode reference.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct ReferenceKind(pub u8);

impl ReferenceKind {
    /// Garbage-collected ownership.
    pub const MANAGED: Self = Self(0);
    /// Unique explicit ownership.
    pub const UNIQUE: Self = Self(1);
    /// Non-owning checked access.
    pub const BORROWED: Self = Self(2);
    /// Unchecked access without retention.
    pub const RAW: Self = Self(3);

    /// Return the stable bytecode code.
    pub const fn code(self) -> u8 {
        self.0
    }

    /// Return the reference kind with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "managed" => Some(Self::MANAGED),
            "unique" => Some(Self::UNIQUE),
            "borrowed" => Some(Self::BORROWED),
            "raw" => Some(Self::RAW),
            _ => None,
        }
    }

    /// Return whether this ownership is defined by the bytecode ISA.
    pub const fn is_defined(self) -> bool {
        self.0 <= Self::RAW.0
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> Option<&'static str> {
        match self {
            Self::MANAGED => Some("managed"),
            Self::UNIQUE => Some("unique"),
            Self::BORROWED => Some("borrowed"),
            Self::RAW => Some("raw"),
            _ => None,
        }
    }
}

/// One heap ownership domain.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct Space(u8);

impl Space {
    /// Worker-local runtime storage.
    pub const LOCAL: Self = Self(0);
    /// Runtime-shared storage.
    pub const SHARED: Self = Self(1);

    /// Return the stable bytecode code.
    pub const fn code(self) -> u8 {
        self.0
    }

    /// Decode one stable heap ownership domain.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::LOCAL),
            1 => Some(Self::SHARED),
            _ => None,
        }
    }
    /// Return the space with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "local" => Some(Self::LOCAL),
            "shared" => Some(Self::SHARED),
            _ => None,
        }
    }

    /// Return whether this space is defined by the bytecode ISA.
    pub const fn is_defined(self) -> bool {
        self.0 <= Self::SHARED.0
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> Option<&'static str> {
        match self {
            Self::LOCAL => Some("local"),
            Self::SHARED => Some("shared"),
            _ => None,
        }
    }
}

/// The backing storage named by one bytecode reference.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct Storage(u8);

impl Storage {
    /// Worker-local heap storage.
    pub const LOCAL: Self = Self(0);
    /// Runtime-shared heap storage.
    pub const SHARED: Self = Self(1);
    /// Current activation frame storage.
    pub const FRAME: Self = Self(2);
    /// Immutable Program constant storage.
    pub const CONSTANT: Self = Self(3);
    /// Worker-local global storage.
    pub const LOCAL_GLOBAL: Self = Self(4);
    /// Runtime-shared global storage.
    pub const SHARED_GLOBAL: Self = Self(5);
    /// Storage a borrow resolves by address at runtime.
    pub const ANY: Self = Self(6);

    /// Return the stable bytecode code.
    pub const fn code(self) -> u8 {
        self.0
    }

    /// Decode one stable reference storage.
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::LOCAL),
            1 => Some(Self::SHARED),
            2 => Some(Self::FRAME),
            3 => Some(Self::CONSTANT),
            4 => Some(Self::LOCAL_GLOBAL),
            5 => Some(Self::SHARED_GLOBAL),
            6 => Some(Self::ANY),
            _ => None,
        }
    }

    /// Return the storage with one canonical name.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "local" => Some(Self::LOCAL),
            "shared" => Some(Self::SHARED),
            "frame" => Some(Self::FRAME),
            "constant" => Some(Self::CONSTANT),
            "global" => Some(Self::LOCAL_GLOBAL),
            "shared global" => Some(Self::SHARED_GLOBAL),
            "any" => Some(Self::ANY),
            _ => None,
        }
    }

    /// Return whether this storage is defined by the bytecode ISA.
    pub const fn is_defined(self) -> bool {
        self.0 <= Self::ANY.0
    }

    /// Return the canonical bytecode text name.
    pub const fn name(self) -> Option<&'static str> {
        match self {
            Self::LOCAL => Some("local"),
            Self::SHARED => Some("shared"),
            Self::FRAME => Some("frame"),
            Self::CONSTANT => Some("constant"),
            Self::LOCAL_GLOBAL => Some("global"),
            Self::SHARED_GLOBAL => Some("shared global"),
            Self::ANY => Some("any"),
            _ => None,
        }
    }

    /// Return the storage for one heap ownership domain.
    pub const fn heap(space: Space) -> Self {
        Self(space.0)
    }

    /// Return the heap ownership domain when this is heap storage.
    pub const fn heap_space(self) -> Option<Space> {
        match self {
            Self::LOCAL => Some(Space::LOCAL),
            Self::SHARED => Some(Space::SHARED),
            _ => None,
        }
    }
}

/// The ownership and storage of one bytecode reference.
#[repr(C)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct ReferenceType {
    /// The reference ownership.
    kind: ReferenceKind,
    /// The referenced storage.
    storage: Storage,
}

impl ReferenceType {
    /// Create one reference type.
    pub const fn new(kind: ReferenceKind, storage: Storage) -> Self {
        Self { kind, storage }
    }

    /// Decode one stable reference operand.
    pub const fn from_bits(bits: u16) -> Option<Self> {
        let bytes = bits.to_le_bytes();
        let reference = Self::new(ReferenceKind(bytes[0]), Storage(bytes[1]));

        if reference.is_defined() {
            Some(reference)
        } else {
            None
        }
    }

    /// Encode this reference type into one stable operand.
    pub const fn bits(self) -> u16 {
        u16::from_le_bytes([self.kind.0, self.storage.0])
    }

    /// Return the reference ownership.
    pub const fn kind(self) -> ReferenceKind {
        self.kind
    }

    /// Return the referenced storage.
    pub const fn storage(self) -> Storage {
        self.storage
    }

    /// Return whether this reference type is defined by the bytecode ISA.
    pub const fn is_defined(self) -> bool {
        self.kind.is_defined() && self.storage.is_defined()
    }
}

/// The representation category of one bytecode value.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct ValueTag(pub u8);

impl ValueTag {
    /// One scalar value of at most 64 bits.
    pub const SCALAR: Self = Self(0);
    /// One signed 128-bit integer value.
    pub const INT128: Self = Self(1);
    /// One unsigned 128-bit integer value.
    pub const UINT128: Self = Self(2);
    /// One dense runtime type id.
    pub const TYPE_ID: Self = Self(3);
    /// One initialized Destack reference.
    pub const REFERENCE: Self = Self(4);
    /// One uninitialized Destack reference token.
    pub const UNINIT_REFERENCE: Self = Self(5);
    /// One unowned native pointer.
    pub const POINTER: Self = Self(6);
    /// One bare callable pointer.
    pub const FUNCTION_POINTER: Self = Self(7);
    /// One callable pointer and captured environment.
    pub const FUNCTION: Self = Self(8);
    /// One contiguous range descriptor.
    pub const SLICE: Self = Self(9);
    /// One uninitialized contiguous range descriptor.
    pub const UNINIT_SLICE: Self = Self(10);
    /// One erased payload and dynamic dispatch table.
    pub const DYNAMIC: Self = Self(11);
    /// One inline fixed-width vector.
    pub const VECTOR: Self = Self(14);
    /// One indexed type spanning contiguous register words.
    pub const INDEXED: Self = Self(15);
    /// No runtime value or register words.
    pub const VOID: Self = Self(16);

    /// Return whether this value category is defined by the bytecode ISA.
    pub const fn is_defined(self) -> bool {
        matches!(self.0, 0..=11 | 14..=16)
    }
}

impl ValueType {
    /// The stable encoded byte length of one value type operand.
    pub const BYTE_LEN: usize = 12;

    /// Create one scalar value type.
    pub const fn scalar(scalar: Scalar) -> Self {
        Self::new(ValueTag::SCALAR, scalar.code(), 1, 0, 0)
    }

    /// Create one signed 128-bit integer value type.
    pub const fn int128() -> Self {
        Self::new(ValueTag::INT128, 0, 2, 0, 0)
    }

    /// Create one unsigned 128-bit integer value type.
    pub const fn uint128() -> Self {
        Self::new(ValueTag::UINT128, 0, 2, 0, 0)
    }

    /// Create one dense runtime type id value type.
    pub const fn type_id() -> Self {
        Self::new(ValueTag::TYPE_ID, 0, 1, 0, 0)
    }

    /// Create one initialized reference value type.
    pub const fn reference(kind: ReferenceKind, storage: Storage) -> Self {
        Self::new_reference(ValueTag::REFERENCE, ReferenceType::new(kind, storage))
    }

    /// Create one uninitialized reference token value type.
    pub const fn uninit_reference(kind: ReferenceKind, storage: Storage) -> Self {
        Self::new_reference(
            ValueTag::UNINIT_REFERENCE,
            ReferenceType::new(kind, storage),
        )
    }

    /// Create one unowned native pointer value type.
    pub const fn pointer() -> Self {
        Self::new(ValueTag::POINTER, 0, 1, 0, 0)
    }

    /// Create one bare callable pointer value type.
    pub const fn function_pointer() -> Self {
        Self::new(ValueTag::FUNCTION_POINTER, 0, 1, 0, 0)
    }

    /// Create one callable pointer and environment pair.
    pub const fn function(reference: ReferenceType) -> Self {
        Self {
            tag: ValueTag::FUNCTION,
            scalar: 0,
            reference,
            word_count: 2,
            lane_count: 0,
            type_id: 0,
        }
    }

    /// Create one initialized slice value type.
    pub const fn slice(element: TypeId, kind: ReferenceKind, storage: Storage) -> Self {
        Self::new_slice(ValueTag::SLICE, element, ReferenceType::new(kind, storage))
    }

    /// Create one uninitialized slice value type.
    pub const fn uninit_slice(element: TypeId, kind: ReferenceKind, storage: Storage) -> Self {
        Self::new_slice(
            ValueTag::UNINIT_SLICE,
            element,
            ReferenceType::new(kind, storage),
        )
    }

    /// Create one dynamic value type.
    pub const fn dynamic(constraint: TypeId, reference: ReferenceType) -> Self {
        Self {
            tag: ValueTag::DYNAMIC,
            scalar: 0,
            reference,
            word_count: 2,
            lane_count: 0,
            type_id: constraint.0,
        }
    }

    /// Create one inline fixed-width vector value type.
    pub const fn vector(ty: VectorType) -> Self {
        Self::new(
            ValueTag::VECTOR,
            ty.scalar.code(),
            ty.word_count(),
            ty.lane_count,
            0,
        )
    }

    /// Create one indexed value type.
    pub const fn indexed(ty: TypeId, word_count: u16) -> Self {
        Self::new(ValueTag::INDEXED, 0, word_count, 0, ty.0)
    }

    /// Create the zero-width void type.
    pub const fn void() -> Self {
        Self::new(ValueTag::VOID, 0, 0, 0, 0)
    }

    /// Return the value representation category.
    pub const fn tag(self) -> ValueTag {
        self.tag
    }

    /// Return the contiguous register word count.
    pub const fn word_count(self) -> u16 {
        self.word_count
    }

    /// Return whether this value has one runtime profile key.
    pub const fn is_profile_value(self) -> bool {
        matches!(
            self.tag,
            ValueTag::SCALAR
                | ValueTag::TYPE_ID
                | ValueTag::REFERENCE
                | ValueTag::POINTER
                | ValueTag::FUNCTION_POINTER
        )
    }

    /// Return the scalar representation for a scalar value.
    pub const fn scalar_type(self) -> Option<Scalar> {
        if self.tag.0 == ValueTag::SCALAR.0 {
            Scalar::from_code(self.scalar)
        } else {
            None
        }
    }

    /// Return whether this is a signed integer value.
    pub const fn is_signed_integer(self) -> bool {
        if self.tag.0 == ValueTag::INT128.0 {
            return true;
        }

        matches!(
            self.scalar_type(),
            Some(Scalar::Int8 | Scalar::Int16 | Scalar::Int32 | Scalar::Int64)
        )
    }

    /// Return whether this is an unsigned integer value.
    pub const fn is_unsigned_integer(self) -> bool {
        if self.tag.0 == ValueTag::UINT128.0 {
            return true;
        }

        matches!(
            self.scalar_type(),
            Some(Scalar::Uint8 | Scalar::Uint16 | Scalar::Uint32 | Scalar::Uint64)
        )
    }

    /// Return the vector type for a vector value.
    pub const fn vector_type(self) -> Option<VectorType> {
        if self.tag.0 != ValueTag::VECTOR.0 {
            return None;
        }
        let Some(scalar) = Scalar::from_code(self.scalar) else {
            return None;
        };

        Some(VectorType::new(scalar, self.lane_count))
    }

    /// Return the type of an initialized or uninitialized reference.
    pub const fn reference_type(self) -> Option<ReferenceType> {
        if self.is_reference() {
            Some(self.reference)
        } else {
            None
        }
    }

    /// Return whether this is an uninitialized reference token.
    pub const fn is_uninitialized_reference(self) -> bool {
        self.tag.0 == ValueTag::UNINIT_REFERENCE.0
    }

    /// Return whether this is an uninitialized reference or slice value.
    pub const fn is_uninitialized(self) -> bool {
        self.tag.0 == ValueTag::UNINIT_REFERENCE.0 || self.tag.0 == ValueTag::UNINIT_SLICE.0
    }

    /// Return the corresponding initialized value type.
    pub const fn initialized(self) -> Option<Self> {
        if self.tag.0 == ValueTag::UNINIT_REFERENCE.0 {
            Some(Self::new_reference(ValueTag::REFERENCE, self.reference))
        } else if self.tag.0 == ValueTag::UNINIT_SLICE.0 {
            Some(Self::new_slice(
                ValueTag::SLICE,
                TypeId(self.type_id),
                self.reference,
            ))
        } else {
            None
        }
    }

    /// Return whether this is an initialized reference.
    pub const fn is_initialized_reference(self) -> bool {
        self.tag.0 == ValueTag::REFERENCE.0
    }

    /// Return whether this is an unowned native pointer.
    pub const fn is_pointer(self) -> bool {
        self.tag.0 == ValueTag::POINTER.0
    }

    /// Return whether this is a bare function pointer.
    pub const fn is_function_pointer(self) -> bool {
        self.tag.0 == ValueTag::FUNCTION_POINTER.0
    }

    /// Return whether this is a function value.
    pub const fn is_function(self) -> bool {
        self.tag.0 == ValueTag::FUNCTION.0
    }

    /// Return the captured environment reference carried by a function value.
    pub const fn function_reference(self) -> Option<ReferenceType> {
        if self.is_function() {
            Some(self.reference)
        } else {
            None
        }
    }

    /// Return the element type for a slice value.
    pub const fn slice_element(self) -> Option<TypeId> {
        if self.is_slice() {
            Some(TypeId(self.type_id))
        } else {
            None
        }
    }

    /// Return the reference type carried by a slice value.
    pub const fn slice_reference(self) -> Option<ReferenceType> {
        if self.is_slice() {
            Some(self.reference)
        } else {
            None
        }
    }

    /// Return whether this is an initialized or uninitialized slice value.
    pub const fn is_slice(self) -> bool {
        self.tag.0 == ValueTag::SLICE.0 || self.tag.0 == ValueTag::UNINIT_SLICE.0
    }

    /// Return the constraint type for a dynamic value.
    pub const fn constraint(self) -> Option<TypeId> {
        if self.tag.0 == ValueTag::DYNAMIC.0 {
            Some(TypeId(self.type_id))
        } else {
            None
        }
    }

    /// Return the erased payload reference carried by a dynamic value.
    pub const fn dynamic_reference(self) -> Option<ReferenceType> {
        if self.tag.0 == ValueTag::DYNAMIC.0 {
            Some(self.reference)
        } else {
            None
        }
    }

    /// Return whether this is a dynamic value.
    pub const fn is_dynamic(self) -> bool {
        self.tag.0 == ValueTag::DYNAMIC.0
    }

    /// Return the indexed runtime type.
    pub const fn indexed_type(self) -> Option<TypeId> {
        if self.tag.0 == ValueTag::INDEXED.0 {
            Some(TypeId(self.type_id))
        } else {
            None
        }
    }

    /// Return whether this is an indexed value type.
    pub const fn is_indexed(self) -> bool {
        self.tag.0 == ValueTag::INDEXED.0
    }

    /// Replace the embedded object-local type.
    pub fn map_type<E>(
        self,
        map_type: impl FnOnce(TypeId) -> Result<TypeId, E>,
    ) -> Result<Self, E> {
        let type_id = if self.is_slice() || self.is_dynamic() || self.is_indexed() {
            map_type(TypeId(self.type_id))?.0
        } else {
            self.type_id
        };

        Ok(Self { type_id, ..self })
    }

    /// Return whether every encoded field is canonical and defined by the ISA.
    pub const fn is_defined(self) -> bool {
        if !self.tag.is_defined() {
            return false;
        }

        match self.tag {
            ValueTag::SCALAR => {
                Scalar::from_code(self.scalar).is_some()
                    && self.word_count == 1
                    && self.has_no_qualifiers()
            }
            ValueTag::INT128 | ValueTag::UINT128 => {
                self.scalar == 0 && self.word_count == 2 && self.has_no_qualifiers()
            }
            ValueTag::TYPE_ID | ValueTag::POINTER => {
                self.scalar == 0 && self.word_count == 1 && self.has_no_qualifiers()
            }
            ValueTag::REFERENCE | ValueTag::UNINIT_REFERENCE => {
                self.scalar == 0
                    && self.word_count == 1
                    && self.reference.is_defined()
                    && self.lane_count == 0
                    && self.type_id == 0
            }
            ValueTag::FUNCTION_POINTER => {
                self.scalar == 0 && self.word_count == 1 && self.has_no_qualifiers()
            }
            ValueTag::FUNCTION => {
                self.scalar == 0
                    && self.word_count == 2
                    && self.reference.is_defined()
                    && self.lane_count == 0
                    && self.type_id == 0
            }
            ValueTag::DYNAMIC => {
                self.scalar == 0
                    && self.word_count == 2
                    && self.reference.is_defined()
                    && self.lane_count == 0
            }
            ValueTag::SLICE | ValueTag::UNINIT_SLICE => {
                self.scalar == 0
                    && self.word_count == 2
                    && self.reference.is_defined()
                    && self.lane_count == 0
            }
            ValueTag::VECTOR => {
                let Some(ty) = self.vector_type() else {
                    return false;
                };

                self.word_count == ty.word_count()
                    && self.reference.bits() == 0
                    && self.type_id == 0
            }
            ValueTag::INDEXED => {
                self.scalar == 0
                    && self.word_count != 0
                    && self.lane_count == 0
                    && self.reference.bits() == 0
            }
            ValueTag::VOID => self.scalar == 0 && self.word_count == 0 && self.has_no_qualifiers(),
            _ => false,
        }
    }

    /// Encode this value type into its stable little-endian operand bytes.
    pub const fn bytes(self) -> [u8; Self::BYTE_LEN] {
        let word_count = self.word_count.to_le_bytes();
        let lane_count = self.lane_count.to_le_bytes();
        let type_id = self.type_id.to_le_bytes();

        [
            self.tag.0,
            self.scalar,
            self.reference.kind.0,
            self.reference.storage.0,
            word_count[0],
            word_count[1],
            lane_count[0],
            lane_count[1],
            type_id[0],
            type_id[1],
            type_id[2],
            type_id[3],
        ]
    }

    /// Decode one stable little-endian value type operand.
    pub const fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Option<Self> {
        let ty = Self {
            tag: ValueTag(bytes[0]),
            scalar: bytes[1],
            reference: ReferenceType::new(ReferenceKind(bytes[2]), Storage(bytes[3])),
            word_count: u16::from_le_bytes([bytes[4], bytes[5]]),
            lane_count: u16::from_le_bytes([bytes[6], bytes[7]]),
            type_id: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        };

        if ty.is_defined() { Some(ty) } else { None }
    }

    /// Create one canonical non-reference value type.
    const fn new(
        tag: ValueTag,
        scalar: u8,
        word_count: u16,
        lane_count: u16,
        type_id: u32,
    ) -> Self {
        Self {
            tag,
            scalar,
            reference: ReferenceType::new(ReferenceKind::MANAGED, Storage::LOCAL),
            word_count,
            lane_count,
            type_id,
        }
    }

    /// Create one canonical reference value type.
    const fn new_reference(tag: ValueTag, reference: ReferenceType) -> Self {
        Self {
            tag,
            scalar: 0,
            reference,
            word_count: 1,
            lane_count: 0,
            type_id: 0,
        }
    }

    /// Create one canonical slice value type.
    const fn new_slice(tag: ValueTag, element: TypeId, reference: ReferenceType) -> Self {
        Self {
            tag,
            scalar: 0,
            reference,
            word_count: 2,
            lane_count: 0,
            type_id: element.0,
        }
    }

    /// Return whether this is an initialized or uninitialized reference.
    const fn is_reference(self) -> bool {
        self.tag.0 == ValueTag::REFERENCE.0 || self.tag.0 == ValueTag::UNINIT_REFERENCE.0
    }

    /// Return whether fields unused by an unqualified value are zero.
    const fn has_no_qualifiers(self) -> bool {
        self.reference.bits() == 0 && self.lane_count == 0 && self.type_id == 0
    }
}

const _: () = assert!(size_of::<ReferenceKind>() == 1);
const _: () = assert!(size_of::<Space>() == 1);
const _: () = assert!(size_of::<Storage>() == 1);
const _: () = assert!(size_of::<ReferenceType>() == 2);
const _: () = assert!(size_of::<ValueTag>() == 1);
const _: () = assert!(size_of::<ValueType>() == 12);
