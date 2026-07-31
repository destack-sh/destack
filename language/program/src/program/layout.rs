use std::num::NonZeroU32;

use destack_bytecode as bytecode;
use destack_core::{
    EntryRange, EntryStore, Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice,
    StringId,
};
use destack_mir::{
    Access, Discriminant, FloatType, GlobalStorage, Nullability, ReferenceKind, Space, Storage,
    TensorFormat, TensorReduction, TensorViewFormat, TraceId, VariantEncoding,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::GlobalAddress;

use super::{SignatureId, TypeId, Word};

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

    /// Return one field by logical source index.
    pub fn field<'a>(
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

    /// Return whether every layout payload range fits its flattened column.
    pub(super) fn ranges_fit(&self, sections: SectionImage<'_>) -> bool {
        let fields = sections.entries(self.fields).len();
        let cases = sections.entries(self.cases).len();
        let dimensions = sections.entries(self.tensor_dimensions).len();
        let axes = sections.entries(self.tensor_axes).len();

        // check each variable layout payload against its owning column
        sections
            .entries(self.layouts)
            .iter()
            .all(|layout| match layout.shape {
                LayoutShape::Struct(range) | LayoutShape::Tuple(range) => range.fits(fields),
                LayoutShape::Tensor(tensor) => {
                    tensor.dimensions.fits(dimensions) && tensor.sharding.fits(axes)
                }
                LayoutShape::TensorView(view) => {
                    view.dimensions.fits(dimensions) && view.sharding.fits(axes)
                }
                LayoutShape::Variant(variant) => variant.cases.fits(cases),
                LayoutShape::Object(object) => object.fields.fits(fields),
                _ => true,
            })
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

    /// Return the scalar storage width in bytes.
    pub const fn byte_len(self) -> usize {
        match self {
            Self::Int { width, .. } => (width as usize).div_ceil(u8::BITS as usize),
            Self::Float {
                format: FloatType::Float16 | FloatType::Bfloat16,
            } => 2,
            Self::Float {
                format: FloatType::Float32,
            }
            | Self::Character => 4,
            Self::Float {
                format: FloatType::Float64,
            } => 8,
            Self::Boolean => 1,
        }
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
    /// Local storage reference.
    LocalReference,
    /// Shared storage reference.
    SharedReference,
    /// Frame storage reference.
    FrameReference,
    /// Global storage reference.
    GlobalReference,
    /// Function pointer.
    FunctionPointer,
}

impl WordLayout {
    /// Return the word layout for one reference.
    #[inline(always)]
    pub fn reference(storage: Storage) -> Self {
        match storage {
            Storage::Heap(Space::Local) => Self::LocalReference,
            Storage::Heap(Space::Shared) => Self::SharedReference,
            Storage::Frame => Self::FrameReference,
            Storage::Global(_) => Self::GlobalReference,
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
            Self::LocalReference
            | Self::SharedReference
            | Self::FrameReference
            | Self::FunctionPointer => pointer_bytes,
            Self::GlobalReference => GlobalAddress::BYTE_LEN,
        }
    }

    /// Decode raw memory bits into one execution word.
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
            Self::LocalReference
            | Self::SharedReference
            | Self::FrameReference
            | Self::GlobalReference
            | Self::FunctionPointer => Word::from_bits(raw),
        }
    }

    /// Encode one execution word into raw memory bits.
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
            | Self::LocalReference
            | Self::SharedReference
            | Self::FrameReference
            | Self::GlobalReference
            | Self::FunctionPointer => value.bits(),
        }
    }

    /// Encode one execution word into its memory-width bytes.
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
    /// Return the word layout when this value fits one execution word.
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
            LayoutShape::Tensor(tensor) => tensor.reference.word_layout(),
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
    Dynamic(DynamicLayout),
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
    /// Packed ownership, storage, access, and nullability.
    bits: u16,
    /// Explicit initialized row padding.
    padding: [u8; 2],
}

impl ReferenceLayout {
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

    /// Create one reference layout.
    pub fn new(
        pointee: TypeId,
        kind: ReferenceKind,
        storage: Storage,
        access: Access,
        nullability: Nullability,
    ) -> Self {
        let kind = match kind {
            ReferenceKind::Managed => 1,
            ReferenceKind::Unique => 2,
            ReferenceKind::Borrowed => 3,
            ReferenceKind::Raw => 4,
        };
        let access = match access {
            Access::Readonly => 0,
            Access::Mutable => 1,
            Access::Exclusive => 2,
        };
        let storage = u16::from(Self::storage_bits(storage));
        let nullability = match nullability {
            Nullability::None => 0,
            Nullability::Null => 1,
            Nullability::Undefined => 2,
            Nullability::NullOrUndefined => 3,
        };

        let mut bits = kind & Self::KIND_MASK;
        bits |= access << Self::ACCESS_SHIFT;
        bits |= nullability << Self::NULLABILITY_SHIFT;
        bits |= storage << Self::STORAGE_SHIFT;

        Self {
            pointee,
            bits,
            padding: [0; 2],
        }
    }

    /// Return the reference kind.
    pub fn kind(self) -> Option<ReferenceKind> {
        match self.bits & Self::KIND_MASK {
            1 => Some(ReferenceKind::Managed),
            2 => Some(ReferenceKind::Unique),
            3 => Some(ReferenceKind::Borrowed),
            4 => Some(ReferenceKind::Raw),
            _ => None,
        }
    }

    /// Return the reference access.
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

    /// Return the storage implied by this reference.
    pub fn storage(self) -> Option<Storage> {
        let bits = ((self.bits >> Self::STORAGE_SHIFT) & Self::STORAGE_MASK) as u8;

        Self::storage_from_bits(bits)
    }

    /// Return the traced heap space when this reference names heap storage.
    pub fn heap_space(self) -> Option<Space> {
        let kind = self.kind()?;
        if kind == ReferenceKind::Raw {
            return None;
        }

        match self.storage()? {
            Storage::Heap(space) => Some(space),
            Storage::Frame | Storage::Global(_) => None,
        }
    }

    /// Return the word layout for this reference.
    pub fn word_layout(self) -> Option<WordLayout> {
        self.kind()?;

        Some(WordLayout::reference(self.storage()?))
    }

    /// Decode reference storage from packed bits.
    fn storage_from_bits(bits: u8) -> Option<Storage> {
        match bits {
            0 => Some(Storage::Heap(Space::Local)),
            1 => Some(Storage::Heap(Space::Shared)),
            2 => Some(Storage::Frame),
            3 => Some(Storage::Global(GlobalStorage::Constant)),
            4 => Some(Storage::Global(GlobalStorage::Local)),
            5 => Some(Storage::Global(GlobalStorage::Shared)),
            _ => None,
        }
    }

    /// Encode reference storage as packed bits.
    fn storage_bits(storage: Storage) -> u8 {
        match storage {
            Storage::Heap(Space::Local) => 0,
            Storage::Heap(Space::Shared) => 1,
            Storage::Frame => 2,
            Storage::Global(GlobalStorage::Constant) => 3,
            Storage::Global(GlobalStorage::Local) => 4,
            Storage::Global(GlobalStorage::Shared) => 5,
        }
    }
}

const _: () = assert!(std::mem::size_of::<ReferenceLayout>() == 8);

/// Concrete layout for one dynamic value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct DynamicLayout {
    /// The accepted runtime type constraint.
    pub constraint: TypeId,
    /// The nullish values accepted by this descriptor.
    pub nullability: Nullability,
}

/// Concrete layout for one closure value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FunctionLayout {
    /// The callable signature.
    pub signature: SignatureId,
    /// The nullish values accepted by this descriptor.
    pub nullability: Nullability,
}

const _: () = assert!(std::mem::size_of::<DynamicLayout>() == 8);
const _: () = assert!(std::mem::size_of::<FunctionLayout>() == 8);

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
    /// The backing tensor storage reference.
    pub reference: ReferenceLayout,
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
    /// The object fields in logical source order.
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

impl TensorSharding {
    /// Return whether the sharding range fits the tensor axis column.
    fn fits(self, axes: usize) -> bool {
        match self {
            Self::Unsharded => true,
            Self::Sharded { axes: range } => range.fits(axes),
        }
    }
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
    Dynamic(DynamicLayout),
    /// Runtime function value storage.
    Function(FunctionLayout),
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
            Self::Dynamic(dynamic) => LayoutShape::Dynamic(dynamic),
            Self::Function(function) => LayoutShape::Function(function),
            Self::Newtype(newtype) => LayoutShape::Newtype(newtype),
        }
    }
}

/// Build-time concrete layout for one object.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectLayoutBuilder {
    /// Byte offset of the virtual table id when present.
    dispatch_offset: Option<u32>,
    /// The object fields in logical source order.
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

/// Build-time concrete layout for a tensor handle.
#[derive(Debug, Clone, PartialEq)]
pub struct TensorLayoutBuilder {
    /// The backing tensor storage reference.
    reference: ReferenceLayout,
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
        reference: ReferenceLayout,
        format: TensorFormat,
        dimensions: impl IntoIterator<Item = TensorDimension>,
    ) -> Self {
        Self {
            reference,
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
            reference: self.reference,
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
        format: TensorViewFormat,
        dimensions: impl IntoIterator<Item = TensorDimension>,
    ) -> Self {
        Self {
            reference,
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
        Access, DiscriminantField, GlobalStorage, Nullability, ReferenceKind, Space, Storage,
        TraceId, VariantEncoding,
    };

    use crate::{
        LayoutBuilder, LayoutId, LayoutShape, LayoutShapeBuilder, LayoutTable, ReferenceLayout,
        TypeId, VariantCaseLayout, VariantLayoutBuilder, WordLayout,
    };

    /// Create one reference layout for reference storage tests.
    fn reference(kind: ReferenceKind, storage: Storage) -> ReferenceLayout {
        ReferenceLayout::new(
            TypeId(1),
            kind,
            storage,
            Access::Readonly,
            Nullability::None,
        )
    }

    /// Managed local references trace local heap storage.
    #[test]
    fn test_reference_layout_traces_local_heap_storage() {
        let reference = reference(ReferenceKind::Managed, Storage::Heap(Space::Local));

        assert_eq!(reference.heap_space(), Some(Space::Local));
        assert_eq!(reference.word_layout(), Some(WordLayout::LocalReference));
    }

    /// Managed shared references trace shared heap storage.
    #[test]
    fn test_reference_layout_traces_shared_heap_storage() {
        let reference = reference(ReferenceKind::Managed, Storage::Heap(Space::Shared));

        assert_eq!(reference.heap_space(), Some(Space::Shared));
        assert_eq!(reference.word_layout(), Some(WordLayout::SharedReference));
    }

    /// Raw references use relative storage coordinates and do not trace heap storage.
    #[test]
    fn test_reference_layout_rejects_raw_heap_tracing() {
        let reference = reference(ReferenceKind::Raw, Storage::Heap(Space::Local));

        assert_eq!(reference.heap_space(), None);
        assert_eq!(reference.word_layout(), Some(WordLayout::LocalReference));
    }

    /// Frame and global references are not heap edges.
    #[test]
    fn test_reference_layout_rejects_frame_and_global_heap_tracing() {
        let frame = reference(ReferenceKind::Borrowed, Storage::Frame);
        let global = reference(
            ReferenceKind::Borrowed,
            Storage::Global(GlobalStorage::Local),
        );

        assert_eq!(frame.heap_space(), None);
        assert_eq!(global.heap_space(), None);
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
        // SAFETY: storage was produced by the SectionBuilder immediately above.
        let sections = unsafe { SectionImage::new(&storage) };
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
