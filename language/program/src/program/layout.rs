use std::num::NonZeroU32;

use destack_core::{
    EntryRange, EntryStore, Optional, SectionEntry, SectionImage, SectionPacker, SectionSlice,
    StringId,
};
use destack_mir::{
    Access, FloatType, Nullability, ReferenceKind, Space, TensorFormat, TensorReduction,
    TensorViewFormat, TraceId,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use crate::GlobalAddress;

use super::{FunctionSignature, Signature, TypeId};

/// Shared layout table for runtime values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct LayoutTable {
    /// Layout entries indexed by LayoutId.
    layouts: SectionSlice<Layout>,
    /// Flattened field layout entries.
    fields: SectionSlice<LayoutField>,
    /// Flattened variant case layout entries.
    variants: SectionSlice<VariantCaseLayout>,
    /// Flattened signature parameter type entries.
    parameters: SectionSlice<TypeId>,
    /// Flattened tensor sharding axis entries.
    tensor_axes: SectionSlice<TensorShardingAxis>,
}

impl LayoutTable {
    /// Pack one layout table.
    pub fn pack(sections: &mut SectionPacker, layouts: Vec<LayoutBuilder>) -> Self {
        let mut entries = Vec::with_capacity(layouts.len());
        let mut fields = EntryStore::new();
        let mut variants = EntryStore::new();
        let mut parameters = EntryStore::new();
        let mut tensor_axes = EntryStore::new();

        // flatten variable layout payloads
        for layout in layouts {
            entries.push(layout.build(
                &mut fields,
                &mut variants,
                &mut parameters,
                &mut tensor_axes,
            ));
        }

        let layouts = sections.insert(entries);
        let fields = sections.insert(fields.into_entries());
        let variants = sections.insert(variants.into_entries());
        let parameters = sections.insert(parameters.into_entries());
        let tensor_axes = sections.insert(tensor_axes.into_entries());

        Self {
            layouts,
            fields,
            variants,
            parameters,
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
            LayoutShape::Struct(fields)
            | LayoutShape::Tuple(fields)
            | LayoutShape::Object(fields) => Some(fields.len as usize),
            _ => None,
        }
    }

    /// Return field layouts for field-addressable shapes.
    pub fn fields<'a>(&self, sections: SectionImage<'a>, layout: &Layout) -> &'a [LayoutField] {
        match layout.shape {
            LayoutShape::Struct(fields)
            | LayoutShape::Tuple(fields)
            | LayoutShape::Object(fields) => fields.slice(sections.entries(self.fields)),
            _ => &[],
        }
    }

    /// Return variant cases for one variant layout.
    pub fn variants<'a>(
        &self,
        sections: SectionImage<'a>,
        layout: VariantLayout,
    ) -> &'a [VariantCaseLayout] {
        layout.variants.slice(sections.entries(self.variants))
    }

    /// Return signature parameters for one function signature.
    pub fn parameters<'a>(
        &self,
        sections: SectionImage<'a>,
        signature: FunctionSignature,
    ) -> &'a [TypeId] {
        signature
            .parameters
            .slice(sections.entries(self.parameters))
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct LayoutId(NonZeroU32);

impl LayoutId {
    /// Create a layout identifier from one raw value.
    #[inline]
    pub const fn new(raw: u32) -> Self {
        match NonZeroU32::new(raw) {
            Some(raw) => Self(raw),
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

/// Scalar storage format for vector and tensor element operations.
#[repr(C, u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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

/// Runtime address space for addressable values.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum AddressSpace {
    /// Local heap storage.
    Local,
    /// Shared heap storage.
    Shared,
    /// Unchecked raw address operations.
    Raw,
    /// Stack pointer.
    Stack,
    /// Frame pointer.
    Frame,
    /// Global address.
    Static,
}

impl AddressSpace {
    /// Return the cell layout used by this address operation family.
    pub const fn cell_layout(self) -> CellLayout {
        match self {
            Self::Local => CellLayout::HeapReference,
            Self::Shared => CellLayout::SharedHeapReference,
            Self::Raw => CellLayout::Address,
            Self::Stack => CellLayout::StackPointer,
            Self::Frame => CellLayout::FramePointer,
            Self::Static => CellLayout::GlobalAddress,
        }
    }
}

/// One-cell storage layout for a scalar or pointer value.
#[repr(C, u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum CellLayout {
    /// Void value.
    Void,
    /// Boolean value.
    Boolean,
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

impl CellLayout {
    /// Return the memory byte width for this cell layout.
    #[inline(always)]
    pub fn byte_len(self, pointer_bytes: usize) -> usize {
        match self {
            Self::Void => 0,
            Self::Boolean => 1,
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
}

/// Storage class for one reference value.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ReferenceStorage {
    /// Worker-local heap.
    Local,
    /// Runtime-shared heap.
    Shared,
    /// Frame bytes.
    Frame,
    /// Static image memory.
    Static,
}

impl From<Space> for ReferenceStorage {
    /// Convert a MIR storage space into a lowered reference storage.
    fn from(space: Space) -> Self {
        match space {
            Space::Local => Self::Local,
            Space::Shared => Self::Shared,
            Space::Frame => Self::Frame,
            Space::Static => Self::Static,
        }
    }
}

impl ReferenceStorage {
    /// Decode reference storage from packed bits.
    pub fn from_bits(bits: u8) -> Option<Self> {
        match bits {
            0 => Some(Self::Local),
            1 => Some(Self::Frame),
            2 => Some(Self::Static),
            3 => Some(Self::Shared),
            _ => None,
        }
    }

    /// Encode reference storage as packed bits.
    pub fn to_bits(self) -> u8 {
        match self {
            Self::Local => 0,
            Self::Frame => 1,
            Self::Static => 2,
            Self::Shared => 3,
        }
    }

    /// Return a human-readable label for diagnostics.
    pub fn label(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Shared => "shared",
            Self::Frame => "frame",
            Self::Static => "static",
        }
    }
}

/// Packed reference flags used by program layouts.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
        let storage_bits = u16::from(ReferenceStorage::from(space).to_bits());
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
    pub fn storage(self) -> Option<ReferenceStorage> {
        let bits = ((self.bits >> Self::STORAGE_SHIFT) & Self::STORAGE_MASK) as u8;

        ReferenceStorage::from_bits(bits)
    }

    /// Return the raw flags bits.
    pub fn bits(self) -> u16 {
        self.bits
    }
}

/// Concrete memory layout for one runtime value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
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
    /// Return the native cell layout for this layout when it fits one VM cell.
    pub fn cell_layout(&self) -> Option<CellLayout> {
        match &self.shape {
            LayoutShape::None => Some(CellLayout::Void),
            LayoutShape::Scalar(ScalarFormat::Boolean) => Some(CellLayout::Boolean),
            LayoutShape::Scalar(ScalarFormat::Int {
                width,
                is_signed: 1,
            }) => u8::try_from(*width)
                .ok()
                .map(|width| CellLayout::Int { width }),
            LayoutShape::Scalar(ScalarFormat::Int {
                width,
                is_signed: 0,
            }) => u8::try_from(*width)
                .ok()
                .map(|width| CellLayout::Uint { width }),
            LayoutShape::Scalar(ScalarFormat::Float {
                format: FloatType::Float16,
            }) => Some(CellLayout::Float16),
            LayoutShape::Scalar(ScalarFormat::Float {
                format: FloatType::Bfloat16,
            }) => Some(CellLayout::Bfloat16),
            LayoutShape::Scalar(ScalarFormat::Float {
                format: FloatType::Float32,
            }) => Some(CellLayout::Float32),
            LayoutShape::Scalar(ScalarFormat::Float {
                format: FloatType::Float64,
            }) => Some(CellLayout::Float64),
            LayoutShape::Reference(reference) => reference.cell_layout(),
            LayoutShape::FunctionPointer(_) => Some(CellLayout::FunctionPointer),
            LayoutShape::Tensor(_) => Some(CellLayout::HeapReference),
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

    /// Return the byte offset of a dynamic dispatch pointer.
    pub const fn dynamic_dispatch_offset(&self) -> Option<u32> {
        match &self.shape {
            LayoutShape::Dynamic => Some(self.alignment),
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub enum LayoutShape {
    /// No runtime storage.
    None,
    /// Builtin scalar storage.
    Scalar(ScalarFormat),
    /// Reference storage.
    Reference(ReferenceLayout),
    /// Function pointer storage.
    FunctionPointer(FunctionSignature),
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
    Object(EntryRange<LayoutField>),
    /// Runtime dynamic value layout.
    Dynamic,
    /// Runtime function value storage.
    Function(FunctionLayout),
    /// Transparent nominal storage.
    Newtype(NewtypeLayout),
}

/// Concrete layout for one slice descriptor.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SliceLayout {
    /// The backing element reference.
    pub reference: ReferenceLayout,
}

/// Concrete layout for one reference value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReferenceLayout {
    /// The referenced value type.
    pub pointee: TypeId,
    /// The packed reference flags.
    pub flags: ReferenceFlags,
}

impl ReferenceLayout {
    /// Return the address space implied by this reference.
    pub fn address_space(&self) -> Option<AddressSpace> {
        Some(match (self.flags.kind()?, self.flags.storage()?) {
            (_, ReferenceStorage::Frame) => AddressSpace::Frame,
            (_, ReferenceStorage::Static) => AddressSpace::Static,
            (ReferenceKind::Raw, ReferenceStorage::Local | ReferenceStorage::Shared) => {
                AddressSpace::Raw
            }
            (_, ReferenceStorage::Local) => AddressSpace::Local,
            (_, ReferenceStorage::Shared) => AddressSpace::Shared,
        })
    }

    /// Return the native cell layout for this reference.
    pub fn cell_layout(&self) -> Option<CellLayout> {
        Some(self.address_space()?.cell_layout())
    }
}

/// Concrete layout for one closure value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionLayout {
    /// The callable signature.
    pub signature: FunctionSignature,
    /// The captured environment type.
    pub environment: TypeId,
}

/// Layout for inline indexed element storage.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorLayout {
    /// The tensor element type.
    pub element: TypeId,
    /// The tensor storage format.
    pub format: TensorFormat,
    /// The tensor placement.
    pub sharding: TensorSharding,
    /// The tensor rank.
    pub rank: u32,
}

/// Concrete layout for a tensor view descriptor.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorViewLayout {
    /// The viewed element type.
    pub element: TypeId,
    /// The tensor view format.
    pub format: TensorViewFormat,
    /// The tensor placement.
    pub sharding: TensorSharding,
    /// The tensor rank.
    pub rank: u32,
}

/// Concrete layout for a variant value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct VariantLayout {
    /// The tag layout.
    pub tag: VariantTagLayout,
    /// The variant payload byte offset.
    pub payload_offset: u32,
    /// The variant cases.
    pub variants: EntryRange<VariantCaseLayout>,
}

/// Concrete layout for a variant tag.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct VariantTagLayout {
    /// The tag type when it has been materialized.
    pub ty: Optional<TypeId>,
    /// The tag size in bytes.
    pub size: u32,
    /// The tag alignment in bytes.
    pub alignment: u32,
}

/// Concrete layout for a nominal newtype.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct NewtypeLayout {
    /// The backing type.
    pub backing_type: TypeId,
    /// The backing type layout.
    pub backing_layout: LayoutId,
}

/// Memory layout for a single field.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
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
    /// Original source index for stable mapping.
    pub source_index: Optional<u32>,
}

/// Concrete layout for one variant case.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct VariantCaseLayout {
    /// The logical case type.
    pub ty: TypeId,
    /// The case layout.
    pub layout: LayoutId,
}

/// Concrete tensor sharding descriptor.
#[repr(C, u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct LayoutBuilder {
    /// The layout shape.
    pub shape: LayoutShapeBuilder,
    /// Total size in bytes, including trailing padding.
    pub size: u32,
    /// Alignment requirement in bytes.
    pub alignment: u32,
    /// Managed-reference trace id for this layout.
    pub trace: TraceId,
}

impl LayoutBuilder {
    /// Build this layout into one section entry.
    fn build(
        self,
        fields: &mut EntryStore<LayoutField>,
        variants: &mut EntryStore<VariantCaseLayout>,
        parameters: &mut EntryStore<TypeId>,
        tensor_axes: &mut EntryStore<TensorShardingAxis>,
    ) -> Layout {
        let shape = self.shape.build(fields, variants, parameters, tensor_axes);

        Layout {
            shape,
            size: self.size,
            alignment: self.alignment,
            trace: self.trace,
        }
    }
}

/// Build-time layout shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum LayoutShapeBuilder {
    /// No runtime storage.
    None,
    /// Builtin scalar storage.
    Scalar(ScalarFormat),
    /// Reference storage.
    Reference(ReferenceLayout),
    /// Function pointer storage.
    FunctionPointer(Signature),
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
    Object(Vec<LayoutField>),
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
        variants: &mut EntryStore<VariantCaseLayout>,
        parameters: &mut EntryStore<TypeId>,
        tensor_axes: &mut EntryStore<TensorShardingAxis>,
    ) -> LayoutShape {
        match self {
            Self::None => LayoutShape::None,
            Self::Scalar(scalar) => LayoutShape::Scalar(scalar),
            Self::Reference(reference) => LayoutShape::Reference(reference),
            Self::FunctionPointer(signature) => {
                LayoutShape::FunctionPointer(build_signature(signature, parameters))
            }
            Self::Struct(layout_fields) => LayoutShape::Struct(fields.append(layout_fields)),
            Self::Tuple(layout_fields) => LayoutShape::Tuple(fields.append(layout_fields)),
            Self::Slice(slice) => LayoutShape::Slice(slice),
            Self::Array(element) => LayoutShape::Array(element),
            Self::Vector(element) => LayoutShape::Vector(element),
            Self::Tensor(tensor) => LayoutShape::Tensor(tensor.build(tensor_axes)),
            Self::TensorView(tensor) => LayoutShape::TensorView(tensor.build(tensor_axes)),
            Self::Variant(variant) => LayoutShape::Variant(VariantLayout {
                tag: variant.tag,
                payload_offset: variant.payload_offset,
                variants: variants.append(variant.variants),
            }),
            Self::Object(layout_fields) => LayoutShape::Object(fields.append(layout_fields)),
            Self::Dynamic => LayoutShape::Dynamic,
            Self::Function(function) => LayoutShape::Function(FunctionLayout {
                signature: build_signature(function.signature, parameters),
                environment: function.environment,
            }),
            Self::Newtype(newtype) => LayoutShape::Newtype(newtype),
        }
    }
}

/// Build-time concrete layout for one closure value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionLayoutBuilder {
    /// The callable signature.
    pub signature: Signature,
    /// The captured environment type.
    pub environment: TypeId,
}

/// Build-time concrete layout for a tensor handle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorLayoutBuilder {
    /// The tensor element type.
    pub element: TypeId,
    /// The tensor storage format.
    pub format: TensorFormat,
    /// The tensor placement.
    pub sharding: TensorShardingBuilder,
    /// The tensor rank.
    pub rank: u32,
}

impl TensorLayoutBuilder {
    /// Build this tensor layout into one section entry.
    fn build(self, tensor_axes: &mut EntryStore<TensorShardingAxis>) -> TensorLayout {
        TensorLayout {
            element: self.element,
            format: self.format,
            sharding: self.sharding.build(tensor_axes),
            rank: self.rank,
        }
    }
}

/// Build-time concrete layout for a tensor view descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorViewLayoutBuilder {
    /// The viewed element type.
    pub element: TypeId,
    /// The tensor view format.
    pub format: TensorViewFormat,
    /// The tensor placement.
    pub sharding: TensorShardingBuilder,
    /// The tensor rank.
    pub rank: u32,
}

impl TensorViewLayoutBuilder {
    /// Build this tensor view layout into one section entry.
    fn build(self, tensor_axes: &mut EntryStore<TensorShardingAxis>) -> TensorViewLayout {
        TensorViewLayout {
            element: self.element,
            format: self.format,
            sharding: self.sharding.build(tensor_axes),
            rank: self.rank,
        }
    }
}

/// Build-time concrete layout for a variant value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct VariantLayoutBuilder {
    /// The tag layout.
    pub tag: VariantTagLayout,
    /// The variant payload byte offset.
    pub payload_offset: u32,
    /// The variant cases.
    pub variants: Vec<VariantCaseLayout>,
}

/// Build-time concrete tensor sharding descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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

fn build_signature(signature: Signature, parameters: &mut EntryStore<TypeId>) -> FunctionSignature {
    FunctionSignature {
        parameters: parameters.append(signature.parameters),
        result: signature.result,
    }
}

// SAFETY: layout ids, layouts, and layout payloads are fixed-width entry values.
unsafe impl SectionEntry for Layout {}
unsafe impl SectionEntry for LayoutId {}
unsafe impl SectionEntry for ScalarFormat {}
unsafe impl SectionEntry for AddressSpace {}
unsafe impl SectionEntry for CellLayout {}
unsafe impl SectionEntry for ReferenceStorage {}
unsafe impl SectionEntry for ReferenceFlags {}
unsafe impl SectionEntry for LayoutShape {}
unsafe impl SectionEntry for SliceLayout {}
unsafe impl SectionEntry for ReferenceLayout {}
unsafe impl SectionEntry for FunctionLayout {}
unsafe impl SectionEntry for ElementLayout {}
unsafe impl SectionEntry for TensorLayout {}
unsafe impl SectionEntry for TensorViewLayout {}
unsafe impl SectionEntry for VariantLayout {}
unsafe impl SectionEntry for VariantTagLayout {}
unsafe impl SectionEntry for NewtypeLayout {}
unsafe impl SectionEntry for LayoutField {}
unsafe impl SectionEntry for VariantCaseLayout {}
unsafe impl SectionEntry for TensorSharding {}
unsafe impl SectionEntry for TensorShardingAxis {}
