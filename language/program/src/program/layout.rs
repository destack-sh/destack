use std::num::NonZeroU32;

use destack_bytecode as bytecode;
use destack_bytecode::Word;
use destack_core::{
    EntryRange, EntryStore, Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice,
    StringId,
};
use destack_mir::{
    Access, Discriminant, FloatType, Nullability, ReferenceKind, Space, TensorFormat,
    TensorReduction, TensorViewFormat, TraceId, VariantEncoding,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::GlobalAddress;

use super::{SignatureId, TypeId, ValueTag};

/// Shared layout table for runtime values.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct LayoutTable {
    /// Layout entries indexed by LayoutId.
    layouts: SectionSlice<Layout>,
    /// Flattened field layout entries.
    fields: SectionSlice<LayoutField>,
    /// Flattened variant case layout entries.
    cases: SectionSlice<VariantCaseLayout>,
    /// Flattened tensor dimensions.
    tensor_dimensions: SectionSlice<TensorDimension>,
    /// Flattened tensor sharding axis entries.
    tensor_axes: SectionSlice<TensorShardingAxis>,
}

impl LayoutTable {
    /// Pack one layout table.
    pub(crate) fn pack(sections: &mut SectionBuilder, layouts: Vec<LayoutBuilder>) -> Self {
        let mut entries = Vec::with_capacity(layouts.len());
        let mut fields = EntryStore::new();
        let mut cases = EntryStore::new();
        let mut tensor_dimensions = EntryStore::new();
        let mut tensor_axes = EntryStore::new();

        // flatten variable layout payloads
        for layout in layouts {
            entries.push(layout.build(
                &mut fields,
                &mut cases,
                &mut tensor_dimensions,
                &mut tensor_axes,
            ));
        }

        let layouts = sections.insert(entries);
        let fields = sections.insert(fields.into_entries());
        let cases = sections.insert(cases.into_entries());
        let tensor_dimensions = sections.insert(tensor_dimensions.into_entries());
        let tensor_axes = sections.insert(tensor_axes.into_entries());

        Self {
            layouts,
            fields,
            cases,
            tensor_dimensions,
            tensor_axes,
        }
    }

    /// Return one layout by id when present.
    pub fn get<'a>(&self, sections: SectionImage<'a>, id: LayoutId) -> Option<&'a Layout> {
        sections.entries(self.layouts).get(id.index())
    }

    /// Return one field by layout index.
    pub fn field_at<'a>(
        &self,
        sections: SectionImage<'a>,
        layout: &Layout,
        index: u32,
    ) -> Option<&'a LayoutField> {
        self.fields(sections, layout).get(index as usize)
    }

    /// Return the field count for field-addressable layouts.
    pub fn field_count(&self, layout: &Layout) -> Option<usize> {
        match layout.shape {
            LayoutShape::Struct(fields) | LayoutShape::Tuple(fields) => Some(fields.len as usize),
            LayoutShape::Object(object) => Some(object.fields.len as usize),
            _ => None,
        }
    }

    /// Return field layouts for field-addressable shapes.
    pub fn fields<'a>(&self, sections: SectionImage<'a>, layout: &Layout) -> &'a [LayoutField] {
        match layout.shape {
            LayoutShape::Struct(fields) | LayoutShape::Tuple(fields) => {
                fields.slice(sections.entries(self.fields))
            }
            LayoutShape::Object(object) => object.fields.slice(sections.entries(self.fields)),
            _ => &[],
        }
    }

    /// Return variant cases for one variant layout.
    pub fn cases<'a>(
        &self,
        sections: SectionImage<'a>,
        layout: VariantLayout,
    ) -> &'a [VariantCaseLayout] {
        layout.cases.slice(sections.entries(self.cases))
    }

    /// Return the dimensions for one tensor layout.
    pub fn tensor_dimensions<'a>(
        &self,
        sections: SectionImage<'a>,
        dimensions: EntryRange<TensorDimension>,
    ) -> &'a [TensorDimension] {
        dimensions.slice(sections.entries(self.tensor_dimensions))
    }

    /// Return tensor sharding axes for one tensor sharding descriptor.
    pub fn tensor_axes<'a>(
        &self,
        sections: SectionImage<'a>,
        sharding: TensorSharding,
    ) -> &'a [TensorShardingAxis] {
        match sharding {
            TensorSharding::Unsharded => &[],
            TensorSharding::Sharded { axes } => axes.slice(sections.entries(self.tensor_axes)),
        }
    }
}

/// Opaque identifier for one program memory layout.
#[repr(transparent)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
pub struct LayoutId(NonZeroU32);

impl LayoutId {
    /// Create a layout identifier when the raw id is non-zero.
    #[inline]
    pub const fn from_raw(raw: u32) -> Option<Self> {
        match NonZeroU32::new(raw) {
            Some(raw) => Some(Self(raw)),
            None => None,
        }
    }

    /// Create a layout identifier from one raw value.
    #[inline]
    pub const fn new(raw: u32) -> Self {
        match Self::from_raw(raw) {
            Some(id) => id,
            None => unreachable!(),
        }
    }

    /// Return the raw non-zero layout identifier value.
    #[inline]
    pub const fn raw(self) -> u32 {
        self.0.get()
    }

    /// Return the zero-based layout table index.
    #[inline]
    pub const fn index(self) -> usize {
        self.raw() as usize - 1
    }
}

/// Scalar storage format.
#[repr(C, u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum ScalarFormat {
    /// Signed or unsigned integers with a bit width.
    Int {
        /// The bit width.
        width: u16,
        /// Whether the integer is signed.
        is_signed: u32,
    },
    /// Floating-point values with a concrete format.
    Float {
        /// The concrete float format.
        format: FloatType,
    },
    /// Boolean values.
    Boolean,
    /// Unicode scalar values.
    Character,
}

impl ScalarFormat {
    /// Create one integer scalar format.
    pub const fn int(width: u16, is_signed: bool) -> Self {
        Self::Int {
            width,
            is_signed: is_signed as u32,
        }
    }

    /// Create one floating-point scalar format.
    pub const fn float(format: FloatType) -> Self {
        Self::Float { format }
    }

    /// Return whether this scalar format is signed.
    pub const fn is_signed(self) -> bool {
        matches!(self, Self::Int { is_signed: 1, .. })
    }
}

impl From<bytecode::Scalar> for ScalarFormat {
    /// Convert one bytecode scalar into its program layout.
    fn from(scalar: bytecode::Scalar) -> Self {
        match scalar {
            bytecode::Scalar::Boolean => Self::Boolean,
            bytecode::Scalar::Int8 => Self::int(8, true),
            bytecode::Scalar::Uint8 => Self::int(8, false),
            bytecode::Scalar::Int16 => Self::int(16, true),
            bytecode::Scalar::Uint16 => Self::int(16, false),
            bytecode::Scalar::Int32 => Self::int(32, true),
            bytecode::Scalar::Uint32 => Self::int(32, false),
            bytecode::Scalar::Int64 => Self::int(64, true),
            bytecode::Scalar::Uint64 => Self::int(64, false),
            bytecode::Scalar::Float16 => Self::float(FloatType::Float16),
            bytecode::Scalar::Bfloat16 => Self::float(FloatType::Bfloat16),
            bytecode::Scalar::Float32 => Self::float(FloatType::Float32),
            bytecode::Scalar::Float64 => Self::float(FloatType::Float64),
        }
    }
}

/// One-word storage layout for a scalar or pointer value.
#[repr(C, u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum WordLayout {
    /// Void value.
    Void,
    /// Boolean value.
    Boolean,
    /// Unicode scalar value.
    Character,
    /// Signed integer value.
    Int { width: u8 },
    /// Unsigned integer value.
    Uint { width: u8 },
    /// Float16 value.
    Float16,
    /// BF16 value.
    Bfloat16,
    /// Float32 value.
    Float32,
    /// Float64 value.
    Float64,
    /// Local heap reference.
    HeapReference,
    /// Shared heap reference.
    SharedHeapReference,
    /// Native address.
    Address,
    /// Stack pointer.
    StackPointer,
    /// Frame pointer.
    FramePointer,
    /// Global address.
    GlobalAddress,
    /// Function pointer.
    FunctionPointer,
}

impl WordLayout {
    /// Return the program value tag accepted by this word layout.
    pub(crate) const fn value_tag(self) -> ValueTag {
        match self {
            Self::Void => ValueTag::Void,
            Self::Boolean => ValueTag::Bool,
            Self::Character => ValueTag::Char,
            Self::Int { .. } => ValueTag::Int,
            Self::Uint { .. } => ValueTag::UInt,
            Self::Float16 => ValueTag::Float16,
            Self::Bfloat16 => ValueTag::Bfloat16,
            Self::Float32 => ValueTag::Float32,
            Self::Float64 => ValueTag::Float64,
            Self::HeapReference => ValueTag::HeapReference,
            Self::SharedHeapReference => ValueTag::SharedHeapReference,
            Self::Address
            | Self::StackPointer
            | Self::FramePointer
            | Self::GlobalAddress
            | Self::FunctionPointer => ValueTag::Address,
        }
    }

    /// Return the word layout for one reference.
    #[inline(always)]
    pub fn reference(space: Space, kind: ReferenceKind) -> Self {
        match (space, kind) {
            (Space::Local | Space::Shared, ReferenceKind::Raw) => Self::Address,
            (Space::Local, _) => Self::HeapReference,
            (Space::Shared, _) => Self::SharedHeapReference,
            (Space::Frame, _) => Self::FramePointer,
            (Space::Static, _) => Self::GlobalAddress,
        }
    }

    /// Return the memory byte width for this word layout.
    #[inline(always)]
    pub fn byte_len(self, pointer_bytes: usize) -> usize {
        match self {
            Self::Void => 0,
            Self::Boolean => 1,
            Self::Character => 4,
            Self::Int { width } | Self::Uint { width } => (width as usize).div_ceil(8),
            Self::Float16 | Self::Bfloat16 => 2,
            Self::Float32 => 4,
            Self::Float64 => 8,
            Self::HeapReference
            | Self::SharedHeapReference
            | Self::Address
            | Self::StackPointer
            | Self::FramePointer
            | Self::FunctionPointer => pointer_bytes,
            Self::GlobalAddress => GlobalAddress::BYTE_LEN,
        }
    }

    /// Decode raw memory bits into one bytecode word.
    #[inline(always)]
    pub fn decode(self, raw: u64) -> Word {
        match self {
            Self::Void => Word::ZERO,
            Self::Boolean => Word::boolean(raw != 0),
            Self::Character => Word::from_bits(raw),
            Self::Int { width } => Word::int(raw as i64, width),
            Self::Uint { width } => Word::uint(raw, width),
            Self::Float16 | Self::Bfloat16 => Word::from_bits(raw),
            Self::Float32 => Word::float32(f32::from_bits(raw as u32)),
            Self::Float64 => Word::float64(f64::from_bits(raw)),
            Self::HeapReference
            | Self::SharedHeapReference
            | Self::Address
            | Self::StackPointer
            | Self::FramePointer
            | Self::GlobalAddress
            | Self::FunctionPointer => Word::from_bits(raw),
        }
    }

    /// Encode one bytecode word into raw memory bits.
    #[inline(always)]
    pub fn encode(self, value: Word) -> u64 {
        match self {
            Self::Void => 0,
            Self::Boolean => u64::from(value.as_boolean()),
            Self::Int { .. }
            | Self::Uint { .. }
            | Self::Character
            | Self::Float16
            | Self::Bfloat16
            | Self::Float32
            | Self::Float64
            | Self::HeapReference
            | Self::SharedHeapReference
            | Self::Address
            | Self::StackPointer
            | Self::FramePointer
            | Self::GlobalAddress
            | Self::FunctionPointer => value.bits(),
        }
    }

    /// Encode one bytecode word into its memory-width bytes.
    pub fn encode_bytes(self, value: Word, pointer_bytes: u8) -> WordBytes {
        let raw = self.encode(value);
        let len = self.byte_len(pointer_bytes as usize);

        WordBytes {
            bytes: raw.to_le_bytes(),
            len,
        }
    }
}

/// One word encoded at its memory width.
#[derive(Debug)]
pub struct WordBytes {
    /// Encoded word bytes.
    bytes: [u8; Word::BYTE_LEN],
    /// Initialized byte count.
    len: usize,
}

impl WordBytes {
    /// Return the encoded bytes.
    pub fn as_slice(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    /// Return the encoded byte count.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Return whether the encoding is empty.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Packed reference flags used by program layouts.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct ReferenceFlags {
    bits: u16,
}

impl ReferenceFlags {
    /// Mask for the packed reference kind.
    const KIND_MASK: u16 = 0x7;
    /// Mask for the packed reference access.
    const ACCESS_MASK: u16 = 0x3;
    /// Mask for the packed reference nullability.
    const NULLABILITY_MASK: u16 = 0x3;
    /// Mask for the packed reference storage.
    const STORAGE_MASK: u16 = 0x7;
    /// Shift for the packed reference access.
    const ACCESS_SHIFT: u8 = 3;
    /// Shift for the packed reference nullability.
    const NULLABILITY_SHIFT: u8 = 5;
    /// Shift for the packed reference storage.
    const STORAGE_SHIFT: u8 = 7;

    /// Empty reference flags.
    pub const NONE: Self = Self { bits: 0 };

    /// Create reference flags from raw bits.
    pub fn from_bits(bits: u16) -> Self {
        Self { bits }
    }

    /// Create reference flags.
    pub fn new(
        kind: ReferenceKind,
        space: Space,
        access: Access,
        nullability: Nullability,
    ) -> Self {
        let kind_bits = match kind {
            ReferenceKind::Managed => 1,
            ReferenceKind::Unique => 2,
            ReferenceKind::Borrowed => 3,
            ReferenceKind::Raw => 4,
        };
        let access_bits = match access {
            Access::Readonly => 0,
            Access::Mutable => 1,
            Access::Exclusive => 2,
        };
        let storage_bits = u16::from(Self::space_bits(space));
        let nullability_bits = match nullability {
            Nullability::None => 0,
            Nullability::Null => 1,
            Nullability::Undefined => 2,
            Nullability::NullOrUndefined => 3,
        };

        let mut bits = kind_bits & Self::KIND_MASK;
        bits |= access_bits << Self::ACCESS_SHIFT;
        bits |= nullability_bits << Self::NULLABILITY_SHIFT;
        bits |= storage_bits << Self::STORAGE_SHIFT;

        Self { bits }
    }

    /// Decode reference storage from packed bits.
    fn space_from_bits(bits: u8) -> Option<Space> {
        match bits {
            0 => Some(Space::Local),
            1 => Some(Space::Frame),
            2 => Some(Space::Static),
            3 => Some(Space::Shared),
            _ => None,
        }
    }

    /// Encode reference storage as packed bits.
    fn space_bits(space: Space) -> u8 {
        match space {
            Space::Local => 0,
            Space::Frame => 1,
            Space::Static => 2,
            Space::Shared => 3,
        }
    }

    /// Return the reference kind when available.
    pub fn kind(self) -> Option<ReferenceKind> {
        match self.bits & Self::KIND_MASK {
            0 => None,
            1 => Some(ReferenceKind::Managed),
            2 => Some(ReferenceKind::Unique),
            3 => Some(ReferenceKind::Borrowed),
            4 => Some(ReferenceKind::Raw),
            _ => None,
        }
    }

    /// Return the reference access when available.
    pub fn access(self) -> Option<Access> {
        self.kind()?;

        match (self.bits >> Self::ACCESS_SHIFT) & Self::ACCESS_MASK {
            0 => Some(Access::Readonly),
            1 => Some(Access::Mutable),
            2 => Some(Access::Exclusive),
            _ => None,
        }
    }

    /// Return the reference nullability.
    pub fn nullability(self) -> Nullability {
        match (self.bits >> Self::NULLABILITY_SHIFT) & Self::NULLABILITY_MASK {
            1 => Nullability::Null,
            2 => Nullability::Undefined,
            3 => Nullability::NullOrUndefined,
            _ => Nullability::None,
        }
    }

    /// Return the reference storage.
    pub fn space(self) -> Option<Space> {
        let bits = ((self.bits >> Self::STORAGE_SHIFT) & Self::STORAGE_MASK) as u8;

        Self::space_from_bits(bits)
    }

    /// Return the raw flags bits.
    pub fn bits(self) -> u16 {
        self.bits
    }
}

/// Concrete memory layout for one runtime value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct Layout {
    /// The layout shape.
    pub shape: LayoutShape,
    /// Total size in bytes, including trailing padding.
    pub size: u32,
    /// Alignment requirement in bytes.
    pub alignment: u32,
    /// Managed-reference trace id for this layout.
    pub trace: TraceId,
}

impl Layout {
    /// Return the word layout when this value fits one bytecode word.
    pub fn word_layout(&self) -> Option<WordLayout> {
        match &self.shape {
            LayoutShape::None => Some(WordLayout::Void),
            LayoutShape::Scalar(ScalarFormat::Boolean) => Some(WordLayout::Boolean),
            LayoutShape::Scalar(ScalarFormat::Character) => Some(WordLayout::Character),
            LayoutShape::Scalar(ScalarFormat::Int {
                width,
                is_signed: 1,
            }) => u8::try_from(*width)
                .ok()
                .map(|width| WordLayout::Int { width }),
            LayoutShape::Scalar(ScalarFormat::Int {
                width,
                is_signed: 0,
            }) => u8::try_from(*width)
                .ok()
                .map(|width| WordLayout::Uint { width }),
            LayoutShape::Scalar(ScalarFormat::Float {
                format: FloatType::Float16,
            }) => Some(WordLayout::Float16),
            LayoutShape::Scalar(ScalarFormat::Float {
                format: FloatType::Bfloat16,
            }) => Some(WordLayout::Bfloat16),
            LayoutShape::Scalar(ScalarFormat::Float {
                format: FloatType::Float32,
            }) => Some(WordLayout::Float32),
            LayoutShape::Scalar(ScalarFormat::Float {
                format: FloatType::Float64,
            }) => Some(WordLayout::Float64),
            LayoutShape::Reference(reference) => reference.word_layout(),
            LayoutShape::FunctionPointer(_) => Some(WordLayout::FunctionPointer),
            LayoutShape::Tensor(_) => Some(WordLayout::HeapReference),
            _ => None,
        }
    }

    /// Return the scalar layout for this layout when it is scalar.
    pub const fn scalar_format(&self) -> Option<ScalarFormat> {
        match &self.shape {
            LayoutShape::Scalar(scalar) => Some(*scalar),
            _ => None,
        }
    }

    /// Return the byte width of this layout.
    pub const fn byte_len(&self) -> usize {
        self.size as usize
    }
}

/// Concrete layout shape.
#[repr(C, u32)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum LayoutShape {
    /// No runtime storage.
    None,
    /// Builtin scalar storage.
    Scalar(ScalarFormat),
    /// Reference storage.
    Reference(ReferenceLayout),
    /// Function pointer storage.
    FunctionPointer(SignatureId),
    /// Struct storage.
    Struct(EntryRange<LayoutField>),
    /// Tuple storage.
    Tuple(EntryRange<LayoutField>),
    /// Slice header storage.
    Slice(SliceLayout),
    /// Fixed array storage.
    Array(ElementLayout),
    /// Vector value storage.
    Vector(ElementLayout),
    /// Tensor handle storage.
    Tensor(TensorLayout),
    /// Tensor view descriptor storage.
    TensorView(TensorViewLayout),
    /// Variant value storage.
    Variant(VariantLayout),
    /// Object storage.
    Object(ObjectLayout),
    /// Runtime dynamic value layout.
    Dynamic,
    /// Runtime function value storage.
    Function(FunctionLayout),
    /// Transparent nominal storage.
    Newtype(NewtypeLayout),
}

/// Concrete layout for one slice descriptor.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct SliceLayout {
    /// The backing element reference.
    pub reference: ReferenceLayout,
}

/// Concrete layout for one reference value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct ReferenceLayout {
    /// The referenced value type.
    pub pointee: TypeId,
    /// The packed reference flags.
    pub flags: ReferenceFlags,
}

impl ReferenceLayout {
    /// Return the storage space implied by this reference.
    pub fn space(&self) -> Option<Space> {
        self.flags.space()
    }

    /// Return the traced heap space when this reference names heap storage.
    pub fn heap_space(&self) -> Option<Space> {
        let kind = self.flags.kind()?;
        if kind == ReferenceKind::Raw {
            return None;
        }

        match self.space()? {
            Space::Local => Some(Space::Local),
            Space::Shared => Some(Space::Shared),
            Space::Frame | Space::Static => None,
        }
    }

    /// Return the word layout for this reference.
    pub fn word_layout(&self) -> Option<WordLayout> {
        Some(WordLayout::reference(self.space()?, self.flags.kind()?))
    }
}

/// Concrete layout for one closure value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FunctionLayout {
    /// The callable signature.
    pub signature: SignatureId,
    /// The captured environment type.
    pub environment: TypeId,
}

/// Layout for inline indexed element storage.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct ElementLayout {
    /// The stored element type.
    pub element: TypeId,
    /// The byte stride between elements.
    pub stride: u32,
    /// The fixed element count.
    pub count: u32,
}

/// Concrete layout for a tensor handle.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct TensorLayout {
    /// The tensor storage space.
    pub space: Space,
    /// The tensor element type.
    pub element: TypeId,
    /// The tensor storage format.
    pub format: TensorFormat,
    /// The tensor placement.
    pub sharding: TensorSharding,
    /// The tensor dimensions.
    pub dimensions: EntryRange<TensorDimension>,
}

/// Concrete layout for a tensor view descriptor.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct TensorViewLayout {
    /// The backing tensor storage reference.
    pub reference: ReferenceLayout,
    /// The viewed element type.
    pub element: TypeId,
    /// The tensor view format.
    pub format: TensorViewFormat,
    /// The tensor placement.
    pub sharding: TensorSharding,
    /// The tensor dimensions.
    pub dimensions: EntryRange<TensorDimension>,
}

/// One tensor dimension.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct TensorDimension {
    /// Whether the dimension is fixed or dynamic.
    pub kind: TensorDimensionKind,
    /// The static extent when present.
    pub extent: u64,
}

impl TensorDimension {
    /// Create one static tensor dimension.
    pub const fn fixed(extent: u64) -> Self {
        Self {
            kind: TensorDimensionKind::Fixed,
            extent,
        }
    }

    /// Create one runtime tensor dimension.
    pub const fn dynamic() -> Self {
        Self {
            kind: TensorDimensionKind::Dynamic,
            extent: 0,
        }
    }

    /// Return the fixed extent when this dimension is static.
    pub const fn fixed_extent(self) -> Option<u64> {
        match self.kind {
            TensorDimensionKind::Fixed => Some(self.extent),
            TensorDimensionKind::Dynamic => None,
        }
    }
}

/// Tensor dimension kind.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum TensorDimensionKind {
    /// A compile-time fixed extent.
    Fixed,
    /// A runtime-provided extent.
    Dynamic,
}

/// Concrete layout for a variant value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct VariantLayout {
    /// The logical discriminant type.
    pub discriminant: TypeId,
    /// The logical payload storage type.
    pub storage: TypeId,
    /// The physical discriminant encoding.
    pub encoding: VariantEncoding,
    /// The variant cases.
    pub cases: EntryRange<VariantCaseLayout>,
}

/// Concrete layout for one object.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct ObjectLayout {
    /// Byte offset of the virtual table id when present.
    pub dispatch_offset: Optional<u32>,
    /// The object fields in physical layout order.
    pub fields: EntryRange<LayoutField>,
}

const _: () = assert!(std::mem::size_of::<VariantLayout>() <= 64);

/// Concrete layout for a nominal newtype.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct NewtypeLayout {
    /// The backing type.
    pub backing_type: TypeId,
    /// The backing type layout.
    pub backing_layout: LayoutId,
}

/// Memory layout for a single field.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct LayoutField {
    /// Field name for lookup and debugging.
    pub name: Optional<StringId>,
    /// Program type of the field.
    pub ty: TypeId,
    /// Byte offset from the start of the aggregate.
    pub offset: u32,
    /// Size of the field in bytes.
    pub size: u32,
    /// Alignment requirement of the field in bytes.
    pub alignment: u32,
    /// Logical field index before physical layout ordering.
    pub source_index: u32,
}

/// Concrete layout for one variant case.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct VariantCaseLayout {
    /// The logical discriminant value.
    pub discriminant: Discriminant,
    /// The logical case type.
    pub ty: TypeId,
    /// The case payload byte offset.
    pub payload_offset: u32,
}

const _: () = assert!(std::mem::size_of::<VariantCaseLayout>() <= 24);

/// Concrete tensor sharding descriptor.
#[repr(C, u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub enum TensorSharding {
    /// Tensor storage is not partitioned across a mesh.
    Unsharded,
    /// Tensor storage is mapped across a mesh axis by axis.
    Sharded {
        /// The per-axis placement descriptors.
        axes: EntryRange<TensorShardingAxis>,
    },
}

/// Per-axis placement descriptor for a sharded tensor.
#[repr(C, u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
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

/// Build-time memory layout.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutBuilder {
    /// The layout shape.
    shape: LayoutShapeBuilder,
    /// Total size in bytes, including trailing padding.
    size: u32,
    /// Alignment requirement in bytes.
    alignment: u32,
    /// Managed-reference trace id for this layout.
    trace: TraceId,
}

impl LayoutBuilder {
    /// Create one memory layout builder.
    pub fn new(shape: LayoutShapeBuilder, size: u32, alignment: u32, trace: TraceId) -> Self {
        Self {
            shape,
            size,
            alignment,
            trace,
        }
    }

    /// Return the total byte length.
    pub const fn byte_len(&self) -> u32 {
        self.size
    }

    /// Return the required byte alignment.
    pub const fn alignment(&self) -> u32 {
        self.alignment
    }

    /// Build this layout into one section entry.
    fn build(
        self,
        fields: &mut EntryStore<LayoutField>,
        cases: &mut EntryStore<VariantCaseLayout>,
        tensor_dimensions: &mut EntryStore<TensorDimension>,
        tensor_axes: &mut EntryStore<TensorShardingAxis>,
    ) -> Layout {
        let shape = self
            .shape
            .build(fields, cases, tensor_dimensions, tensor_axes);

        Layout {
            shape,
            size: self.size,
            alignment: self.alignment,
            trace: self.trace,
        }
    }
}

/// Build-time layout shape.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutShapeBuilder {
    /// No runtime storage.
    None,
    /// Builtin scalar storage.
    Scalar(ScalarFormat),
    /// Reference storage.
    Reference(ReferenceLayout),
    /// Function pointer storage.
    FunctionPointer(SignatureId),
    /// Struct storage.
    Struct(Vec<LayoutField>),
    /// Tuple storage.
    Tuple(Vec<LayoutField>),
    /// Slice header storage.
    Slice(SliceLayout),
    /// Fixed array storage.
    Array(ElementLayout),
    /// Vector value storage.
    Vector(ElementLayout),
    /// Tensor handle storage.
    Tensor(TensorLayoutBuilder),
    /// Tensor view descriptor storage.
    TensorView(TensorViewLayoutBuilder),
    /// Variant value storage.
    Variant(VariantLayoutBuilder),
    /// Object storage.
    Object(ObjectLayoutBuilder),
    /// Runtime dynamic value layout.
    Dynamic,
    /// Runtime function value storage.
    Function(FunctionLayoutBuilder),
    /// Transparent nominal storage.
    Newtype(NewtypeLayout),
}

impl LayoutShapeBuilder {
    /// Build this shape into one section entry.
    fn build(
        self,
        fields: &mut EntryStore<LayoutField>,
        cases: &mut EntryStore<VariantCaseLayout>,
        tensor_dimensions: &mut EntryStore<TensorDimension>,
        tensor_axes: &mut EntryStore<TensorShardingAxis>,
    ) -> LayoutShape {
        match self {
            Self::None => LayoutShape::None,
            Self::Scalar(scalar) => LayoutShape::Scalar(scalar),
            Self::Reference(reference) => LayoutShape::Reference(reference),
            Self::FunctionPointer(signature) => LayoutShape::FunctionPointer(signature),
            Self::Struct(layout_fields) => LayoutShape::Struct(fields.append(layout_fields)),
            Self::Tuple(layout_fields) => LayoutShape::Tuple(fields.append(layout_fields)),
            Self::Slice(slice) => LayoutShape::Slice(slice),
            Self::Array(element) => LayoutShape::Array(element),
            Self::Vector(element) => LayoutShape::Vector(element),
            Self::Tensor(tensor) => {
                LayoutShape::Tensor(tensor.build(tensor_dimensions, tensor_axes))
            }
            Self::TensorView(tensor) => {
                LayoutShape::TensorView(tensor.build(tensor_dimensions, tensor_axes))
            }
            Self::Variant(variant) => LayoutShape::Variant(VariantLayout {
                discriminant: variant.discriminant,
                storage: variant.storage,
                encoding: variant.encoding,
                cases: cases.append(variant.cases),
            }),
            Self::Object(object) => LayoutShape::Object(ObjectLayout {
                dispatch_offset: Optional::from(object.dispatch_offset),
                fields: fields.append(object.fields),
            }),
            Self::Dynamic => LayoutShape::Dynamic,
            Self::Function(function) => LayoutShape::Function(FunctionLayout {
                signature: function.signature,
                environment: function.environment,
            }),
            Self::Newtype(newtype) => LayoutShape::Newtype(newtype),
        }
    }
}

/// Build-time concrete layout for one object.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectLayoutBuilder {
    /// Byte offset of the virtual table id when present.
    dispatch_offset: Option<u32>,
    /// The object fields in physical layout order.
    fields: Vec<LayoutField>,
}

impl ObjectLayoutBuilder {
    /// Create one object layout builder.
    pub fn new(fields: impl IntoIterator<Item = LayoutField>) -> Self {
        Self {
            dispatch_offset: None,
            fields: fields.into_iter().collect(),
        }
    }

    /// Set the virtual table id byte offset.
    pub fn dispatch_offset(mut self, offset: u32) -> Self {
        self.dispatch_offset = Some(offset);

        self
    }
}

/// Build-time concrete layout for one closure value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionLayoutBuilder {
    /// The callable signature.
    signature: SignatureId,
    /// The captured environment type.
    environment: TypeId,
}

impl FunctionLayoutBuilder {
    /// Create one closure layout builder.
    pub fn new(signature: SignatureId, environment: TypeId) -> Self {
        Self {
            signature,
            environment,
        }
    }
}

/// Build-time concrete layout for a tensor handle.
#[derive(Debug, Clone, PartialEq)]
pub struct TensorLayoutBuilder {
    /// The tensor storage space.
    space: Space,
    /// The tensor element type.
    element: TypeId,
    /// The tensor storage format.
    format: TensorFormat,
    /// The tensor placement.
    sharding: TensorShardingBuilder,
    /// The tensor dimensions.
    dimensions: Vec<TensorDimension>,
}

impl TensorLayoutBuilder {
    /// Create one tensor layout builder.
    pub fn new(
        space: Space,
        element: TypeId,
        format: TensorFormat,
        dimensions: impl IntoIterator<Item = TensorDimension>,
    ) -> Self {
        Self {
            space,
            element,
            format,
            sharding: TensorShardingBuilder::Unsharded,
            dimensions: dimensions.into_iter().collect(),
        }
    }

    /// Set the tensor placement.
    pub fn sharding(mut self, sharding: TensorShardingBuilder) -> Self {
        self.sharding = sharding;

        self
    }

    /// Build this tensor layout into one section entry.
    fn build(
        self,
        tensor_dimensions: &mut EntryStore<TensorDimension>,
        tensor_axes: &mut EntryStore<TensorShardingAxis>,
    ) -> TensorLayout {
        TensorLayout {
            space: self.space,
            element: self.element,
            format: self.format,
            sharding: self.sharding.build(tensor_axes),
            dimensions: tensor_dimensions.append(self.dimensions),
        }
    }
}

/// Build-time concrete layout for a tensor view descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct TensorViewLayoutBuilder {
    /// The backing tensor storage reference.
    reference: ReferenceLayout,
    /// The viewed element type.
    element: TypeId,
    /// The tensor view format.
    format: TensorViewFormat,
    /// The tensor placement.
    sharding: TensorShardingBuilder,
    /// The tensor dimensions.
    dimensions: Vec<TensorDimension>,
}

impl TensorViewLayoutBuilder {
    /// Create one tensor view layout builder.
    pub fn new(
        reference: ReferenceLayout,
        element: TypeId,
        format: TensorViewFormat,
        dimensions: impl IntoIterator<Item = TensorDimension>,
    ) -> Self {
        Self {
            reference,
            element,
            format,
            sharding: TensorShardingBuilder::Unsharded,
            dimensions: dimensions.into_iter().collect(),
        }
    }

    /// Set the tensor placement.
    pub fn sharding(mut self, sharding: TensorShardingBuilder) -> Self {
        self.sharding = sharding;

        self
    }

    /// Build this tensor view layout into one section entry.
    fn build(
        self,
        tensor_dimensions: &mut EntryStore<TensorDimension>,
        tensor_axes: &mut EntryStore<TensorShardingAxis>,
    ) -> TensorViewLayout {
        TensorViewLayout {
            reference: self.reference,
            element: self.element,
            format: self.format,
            sharding: self.sharding.build(tensor_axes),
            dimensions: tensor_dimensions.append(self.dimensions),
        }
    }
}

/// Build-time concrete layout for a variant value.
#[derive(Debug, Clone, PartialEq)]
pub struct VariantLayoutBuilder {
    /// The logical discriminant type.
    discriminant: TypeId,
    /// The logical payload storage type.
    storage: TypeId,
    /// The physical discriminant encoding.
    encoding: VariantEncoding,
    /// The variant cases.
    cases: Vec<VariantCaseLayout>,
}

impl VariantLayoutBuilder {
    /// Create one variant layout builder.
    pub fn new(discriminant: TypeId, storage: TypeId, encoding: VariantEncoding) -> Self {
        Self {
            discriminant,
            storage,
            encoding,
            cases: Vec::new(),
        }
    }

    /// Set variant cases.
    pub fn cases(mut self, cases: impl IntoIterator<Item = VariantCaseLayout>) -> Self {
        self.cases = cases.into_iter().collect();

        self
    }
}

/// Build-time concrete tensor sharding descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TensorShardingBuilder {
    /// Tensor storage is not partitioned across a mesh.
    Unsharded,
    /// Tensor storage is mapped across a mesh axis by axis.
    Sharded {
        /// The per-axis placement descriptors.
        axes: Vec<TensorShardingAxis>,
    },
}

impl TensorShardingBuilder {
    /// Build this tensor sharding descriptor into one section entry.
    fn build(self, tensor_axes: &mut EntryStore<TensorShardingAxis>) -> TensorSharding {
        match self {
            Self::Unsharded => TensorSharding::Unsharded,
            Self::Sharded { axes } => TensorSharding::Sharded {
                axes: tensor_axes.append(axes),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use destack_core::{SectionBuilder, SectionImage};
    use destack_mir::{
        Access, DiscriminantField, Nullability, ReferenceKind, Space, TraceId, VariantEncoding,
    };

    use crate::{
        LayoutBuilder, LayoutId, LayoutShape, LayoutShapeBuilder, LayoutTable, ReferenceFlags,
        ReferenceLayout, TypeId, VariantCaseLayout, VariantLayoutBuilder, WordLayout,
    };

    /// Create one reference layout for reference storage tests.
    fn reference(kind: ReferenceKind, space: Space) -> ReferenceLayout {
        ReferenceLayout {
            pointee: TypeId(1),
            flags: ReferenceFlags::new(kind, space, Access::Readonly, Nullability::None),
        }
    }

    /// Managed local references trace local heap storage.
    #[test]
    fn test_reference_layout_traces_local_heap_storage() {
        let reference = reference(ReferenceKind::Managed, Space::Local);

        assert_eq!(reference.heap_space(), Some(Space::Local));
        assert_eq!(reference.word_layout(), Some(WordLayout::HeapReference));
    }

    /// Managed shared references trace shared heap storage.
    #[test]
    fn test_reference_layout_traces_shared_heap_storage() {
        let reference = reference(ReferenceKind::Managed, Space::Shared);

        assert_eq!(reference.heap_space(), Some(Space::Shared));
        assert_eq!(
            reference.word_layout(),
            Some(WordLayout::SharedHeapReference)
        );
    }

    /// Raw references use native address cells and do not trace heap storage.
    #[test]
    fn test_reference_layout_rejects_raw_heap_tracing() {
        let reference = reference(ReferenceKind::Raw, Space::Local);

        assert_eq!(reference.heap_space(), None);
        assert_eq!(reference.word_layout(), Some(WordLayout::Address));
    }

    /// Frame and static references are not heap edges.
    #[test]
    fn test_reference_layout_rejects_frame_and_static_heap_tracing() {
        let frame = reference(ReferenceKind::Borrowed, Space::Frame);
        let static_reference = reference(ReferenceKind::Borrowed, Space::Static);

        assert_eq!(frame.heap_space(), None);
        assert_eq!(static_reference.heap_space(), None);
    }

    /// Preserve niche variant layouts in directly mapped program sections.
    #[test]
    fn test_pack_niche_variant_layout() {
        let encoding = VariantEncoding::Niche {
            field: DiscriminantField::scalar(0, 8),
            untagged_case: 0,
            niche_case_start: 1,
            niche_case_end: 1,
            niche_start: 0u128.into(),
        };
        let layout = LayoutBuilder {
            shape: LayoutShapeBuilder::Variant(VariantLayoutBuilder {
                discriminant: TypeId(1),
                storage: TypeId(2),
                encoding,
                cases: vec![
                    VariantCaseLayout {
                        discriminant: 0u128.into(),
                        ty: TypeId(2),
                        payload_offset: 0,
                    },
                    VariantCaseLayout {
                        discriminant: 1u128.into(),
                        ty: TypeId(3),
                        payload_offset: 0,
                    },
                ],
            }),
            size: 8,
            alignment: 8,
            trace: TraceId::new(1),
        };
        let mut sections = SectionBuilder::new();
        let table = LayoutTable::pack(&mut sections, vec![layout]);
        let storage = sections.build();
        let sections = SectionImage::new(&storage);
        let layout = table
            .get(sections, LayoutId::new(1))
            .expect("variant layout should exist");
        let LayoutShape::Variant(variant) = layout.shape else {
            panic!("layout should remain a variant");
        };

        assert_eq!(variant.encoding, encoding);
        assert_eq!(table.cases(sections, variant).len(), 2);
    }
}
