use std::num::NonZeroU32;

use destack_bytecode as bytecode;
use destack_core::{
    EntryRange, EntryStore, Optional, SectionBuilder, SectionEntry, SectionImage, SectionSlice,
    StringId,
};
use destack_mir::{Access, Discriminant, FloatType, Reference, TraceId, VariantEncoding};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

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
}

impl LayoutTable {
    /// Pack one layout table.
    pub(crate) fn pack(sections: &mut SectionBuilder, layouts: Vec<LayoutBuilder>) -> Self {
        let mut entries = Vec::with_capacity(layouts.len());
        let mut fields = EntryStore::new();
        let mut cases = EntryStore::new();

        // flatten variable layout payloads
        for layout in layouts {
            entries.push(layout.build(&mut fields, &mut cases));
        }

        let layouts = sections.insert(entries);
        let fields = sections.insert(fields.into_entries());
        let cases = sections.insert(cases.into_entries());

        Self {
            layouts,
            fields,
            cases,
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

    /// Return whether every layout payload range fits its flattened column.
    pub(super) fn ranges_fit(&self, sections: SectionImage<'_>) -> bool {
        let fields = sections.entries(self.fields).len();
        let cases = sections.entries(self.cases).len();

        // check each variable layout payload against its owning column
        sections
            .entries(self.layouts)
            .iter()
            .all(|layout| match layout.shape {
                LayoutShape::Struct(range) | LayoutShape::Tuple(range) => range.fits(fields),
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
    /// Float32 value.
    Float32,
    /// Float64 value.
    Float64,
    /// World memory reference, its region classified by address.
    Reference,
    /// Function pointer.
    FunctionPointer,
    /// World-relative raw pointer.
    Pointer,
}

impl WordLayout {
    /// Return the memory byte width for this word layout.
    #[inline(always)]
    pub fn byte_len(self, pointer_bytes: usize) -> usize {
        match self {
            Self::Void => 0,
            Self::Boolean => 1,
            Self::Character => 4,
            Self::Int { width } | Self::Uint { width } => (width as usize).div_ceil(8),
            Self::Float32 => 4,
            Self::Float64 => 8,
            Self::Reference | Self::FunctionPointer | Self::Pointer => pointer_bytes,
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
            Self::Float32 => Word::float32(f32::from_bits(raw as u32)),
            Self::Float64 => Word::float64(f64::from_bits(raw)),
            Self::Reference | Self::FunctionPointer | Self::Pointer => Word::from_bits(raw),
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
            | Self::Float32
            | Self::Float64
            | Self::Reference
            | Self::FunctionPointer
            | Self::Pointer => value.bits(),
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
                format: FloatType::Float32,
            }) => Some(WordLayout::Float32),
            LayoutShape::Scalar(ScalarFormat::Float {
                format: FloatType::Float64,
            }) => Some(WordLayout::Float64),
            LayoutShape::Reference(reference) => reference.word_layout(),
            LayoutShape::FunctionPointer(_) => Some(WordLayout::FunctionPointer),
            LayoutShape::Pointer(_) => Some(WordLayout::Pointer),
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
    /// Process-local machine pointer storage.
    Pointer(PointerLayout),
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
    /// Packed reference kind and access.
    bits: u16,
    /// Explicit initialized entry padding.
    padding: [u8; 2],
}

impl ReferenceLayout {
    /// Mask for the packed reference kind.
    const KIND_MASK: u16 = 0x7;
    /// Mask for the packed reference access.
    const ACCESS_MASK: u16 = 0x3;
    /// Shift for the packed reference access.
    const ACCESS_SHIFT: u8 = 3;

    /// Create one reference layout.
    pub fn new(pointee: TypeId, kind: Reference, access: Access) -> Self {
        let kind = match kind {
            Reference::Managed => 1,
            Reference::Unique => 2,
            Reference::Borrowed => 3,
            Reference::Raw => 4,
        };
        let access = match access {
            Access::Readonly => 0,
            Access::Mutable => 1,
            Access::Immutable => 2,
            Access::Exclusive => 3,
            Access::Parameter(_) => unreachable!("program references close every access"),
        };

        let mut bits = kind & Self::KIND_MASK;
        bits |= access << Self::ACCESS_SHIFT;

        Self {
            pointee,
            bits,
            padding: [0; 2],
        }
    }

    /// Return the reference kind.
    pub fn kind(self) -> Option<Reference> {
        match self.bits & Self::KIND_MASK {
            1 => Some(Reference::Managed),
            2 => Some(Reference::Unique),
            3 => Some(Reference::Borrowed),
            4 => Some(Reference::Raw),
            _ => None,
        }
    }

    /// Return the reference access.
    pub fn access(self) -> Option<Access> {
        self.kind()?;

        match (self.bits >> Self::ACCESS_SHIFT) & Self::ACCESS_MASK {
            0 => Some(Access::Readonly),
            1 => Some(Access::Mutable),
            2 => Some(Access::Immutable),
            3 => Some(Access::Exclusive),
            _ => None,
        }
    }

    /// Return the common word layout for a valid reference.
    pub fn word_layout(self) -> Option<WordLayout> {
        self.kind().map(|_| WordLayout::Reference)
    }
}

const _: () = assert!(std::mem::size_of::<ReferenceLayout>() == 8);

/// Concrete layout for one process-local machine pointer.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct PointerLayout {
    /// The pointed-to value type.
    pub pointee: TypeId,
    /// Packed access.
    bits: u8,
    /// Explicit initialized entry padding.
    padding: [u8; 3],
}

impl PointerLayout {
    /// Mask for the packed pointer access.
    const ACCESS_MASK: u8 = 0x3;

    /// Create one native machine pointer layout.
    pub const fn new(pointee: TypeId, access: Access) -> Self {
        let bits = match access {
            Access::Readonly => 0,
            Access::Mutable => 1,
            Access::Immutable => 2,
            Access::Exclusive => 3,
            Access::Parameter(_) => unreachable!(),
        };

        Self {
            pointee,
            bits,
            padding: [0; 3],
        }
    }

    /// Return the access exposed through this pointer.
    pub const fn access(self) -> Option<Access> {
        match self.bits & Self::ACCESS_MASK {
            0 => Some(Access::Readonly),
            1 => Some(Access::Mutable),
            2 => Some(Access::Immutable),
            3 => Some(Access::Exclusive),
            _ => None,
        }
    }
}

const _: () = assert!(std::mem::size_of::<PointerLayout>() == 8);

/// Concrete layout for one dynamic value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct DynamicLayout {
    /// The accepted runtime type constraint.
    pub constraint: TypeId,
}

/// Concrete layout for one closure value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct FunctionLayout {
    /// The callable signature.
    pub signature: SignatureId,
}

const _: () = assert!(std::mem::size_of::<DynamicLayout>() == 4);
const _: () = assert!(std::mem::size_of::<FunctionLayout>() == 4);

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

/// Concrete layout for a variant value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect, SectionEntry)]
pub struct VariantLayout {
    /// The logical discriminant type.
    pub discriminant: TypeId,
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
    ) -> Layout {
        let shape = self.shape.build(fields, cases);

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
    /// Process-local machine pointer storage.
    Pointer(PointerLayout),
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
    ) -> LayoutShape {
        match self {
            Self::None => LayoutShape::None,
            Self::Scalar(scalar) => LayoutShape::Scalar(scalar),
            Self::Reference(reference) => LayoutShape::Reference(reference),
            Self::Pointer(pointer) => LayoutShape::Pointer(pointer),
            Self::FunctionPointer(signature) => LayoutShape::FunctionPointer(signature),
            Self::Struct(layout_fields) => LayoutShape::Struct(fields.append(layout_fields)),
            Self::Tuple(layout_fields) => LayoutShape::Tuple(fields.append(layout_fields)),
            Self::Slice(slice) => LayoutShape::Slice(slice),
            Self::Array(element) => LayoutShape::Array(element),
            Self::Vector(element) => LayoutShape::Vector(element),
            Self::Variant(variant) => LayoutShape::Variant(VariantLayout {
                discriminant: variant.discriminant,
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

/// Build-time concrete layout for a variant value.
#[derive(Debug, Clone, PartialEq)]
pub struct VariantLayoutBuilder {
    /// The logical discriminant type.
    discriminant: TypeId,
    /// The physical discriminant encoding.
    encoding: VariantEncoding,
    /// The variant cases.
    cases: Vec<VariantCaseLayout>,
}

impl VariantLayoutBuilder {
    /// Create one variant layout builder.
    pub fn new(discriminant: TypeId, encoding: VariantEncoding) -> Self {
        Self {
            discriminant,
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

#[cfg(test)]
mod tests {
    use destack_core::{SectionBuilder, SectionImage};
    use destack_mir::{DiscriminantField, TraceId, VariantEncoding};

    use crate::{
        LayoutBuilder, LayoutId, LayoutShape, LayoutShapeBuilder, LayoutTable, TypeId,
        VariantCaseLayout, VariantLayoutBuilder,
    };

    /// Preserve niche variant layouts in directly mapped program sections.
    #[test]
    fn test_pack_niche_variant_layout() {
        let encoding = VariantEncoding::Niche {
            field: DiscriminantField::scalar(0, 8),
            untagged_case: 0,
            niche_start: 0u128.into(),
        };
        let layout = LayoutBuilder {
            shape: LayoutShapeBuilder::Variant(VariantLayoutBuilder {
                discriminant: TypeId(1),
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
