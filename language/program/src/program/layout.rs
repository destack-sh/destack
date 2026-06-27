use std::num::NonZeroU32;

use destack_core::StringId;
use destack_mir::{
    Access, FloatType, Nullability, ReferenceKind, Space, TensorFormat, TensorSharding,
    TensorViewFormat, TraceId,
};
use destack_serde::Reflect;
use serde::{Deserialize, Serialize};

use super::{Signature, TypeId};

const CELL_BYTE_LEN: usize = std::mem::size_of::<u64>();
const REFERENCE_KIND_MASK: u16 = 0x7;
const REFERENCE_ACCESS_SHIFT: u8 = 3;
const REFERENCE_NULLABILITY_SHIFT: u8 = 5;
const REFERENCE_STORAGE_SHIFT: u8 = 7;

/// Shared executable layout table for all runtime value layouts.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub struct LayoutTable {
    /// Layout entries indexed by LayoutId.
    entries: Vec<Layout>,
}

impl LayoutTable {
    /// Create an empty layout table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert one layout and return its id.
    pub fn insert(&mut self, layout: Layout) -> LayoutId {
        let next_index = self.entries.len() + 1;
        let id = LayoutId::new(next_index as u32);
        self.entries.push(layout);

        id
    }

    /// Resize this table with one repeated layout.
    pub fn resize(&mut self, len: usize, layout: Layout) {
        self.entries.resize(len, layout);
    }

    /// Define one existing layout row.
    pub fn define(&mut self, id: LayoutId, layout: Layout) -> bool {
        let index = id.index();
        if index >= self.entries.len() {
            return false;
        }

        self.entries[index] = layout;

        true
    }

    /// Return one layout by id when present.
    pub fn get(&self, id: LayoutId) -> Option<&Layout> {
        self.entries.get(id.index())
    }

    /// Return one layout by id.
    pub fn layout(&self, id: LayoutId) -> &Layout {
        let index = id.index();
        self.entries
            .get(index)
            .unwrap_or_else(|| unreachable!("missing program layout entry {index}"))
    }
}

/// Opaque identifier for one executable memory layout.
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum ScalarFormat {
    /// Signed or unsigned integers with a bit width.
    Int {
        /// The bit width.
        width: u16,
        /// Whether the integer is signed.
        is_signed: bool,
    },
    /// Floating-point values with a concrete format.
    Float {
        /// The concrete float format.
        format: FloatType,
    },
    /// Boolean values.
    Boolean,
}

/// Runtime address space for addressable values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
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
            Self::GlobalAddress => CELL_BYTE_LEN,
        }
    }
}

/// Storage class for one reference value.
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

/// Packed reference flags used by executable layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ReferenceFlags {
    bits: u16,
}

impl ReferenceFlags {
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

        let mut bits = kind_bits & REFERENCE_KIND_MASK;
        bits |= access_bits << REFERENCE_ACCESS_SHIFT;
        bits |= nullability_bits << REFERENCE_NULLABILITY_SHIFT;
        bits |= storage_bits << REFERENCE_STORAGE_SHIFT;

        Self { bits }
    }

    /// Return the reference kind when available.
    pub fn kind(self) -> Option<ReferenceKind> {
        match self.bits & REFERENCE_KIND_MASK {
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

        match (self.bits >> REFERENCE_ACCESS_SHIFT) & 0x3 {
            0 => Some(Access::Readonly),
            1 => Some(Access::Mutable),
            2 => Some(Access::Exclusive),
            _ => None,
        }
    }

    /// Return the reference nullability.
    pub fn nullability(self) -> Nullability {
        match (self.bits >> REFERENCE_NULLABILITY_SHIFT) & 0x3 {
            1 => Nullability::Null,
            2 => Nullability::Undefined,
            3 => Nullability::NullOrUndefined,
            _ => Nullability::None,
        }
    }

    /// Return the reference storage.
    pub fn storage(self) -> Option<ReferenceStorage> {
        let bits = ((self.bits >> REFERENCE_STORAGE_SHIFT) & 0x7) as u8;

        ReferenceStorage::from_bits(bits)
    }

    /// Return the raw flags bits.
    pub fn bits(self) -> u16 {
        self.bits
    }
}

/// Concrete executable memory layout for one runtime value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
                is_signed: true,
            }) => u8::try_from(*width)
                .ok()
                .map(|width| CellLayout::Int { width }),
            LayoutShape::Scalar(ScalarFormat::Int {
                width,
                is_signed: false,
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

    /// Return one field by layout index.
    pub fn field_at(&self, index: u32) -> Option<&LayoutField> {
        self.shape.fields().get(index as usize)
    }

    /// Return the field count for field-addressable layouts.
    pub fn field_count(&self) -> Option<usize> {
        match &self.shape {
            LayoutShape::Struct(layout) => Some(layout.fields.len()),
            LayoutShape::Tuple(layout) => Some(layout.elements.len()),
            LayoutShape::Object(layout) => Some(layout.fields.len()),
            _ => None,
        }
    }
}

/// Concrete executable layout shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum LayoutShape {
    /// No runtime storage.
    None,
    /// Builtin scalar storage.
    Scalar(ScalarFormat),
    /// Reference storage.
    Reference(ReferenceLayout),
    /// Function pointer storage.
    FunctionPointer(Signature),
    /// Struct storage.
    Struct(StructLayout),
    /// Tuple storage.
    Tuple(TupleLayout),
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

impl LayoutShape {
    /// Return field layouts for field-addressable shapes.
    pub fn fields(&self) -> &[LayoutField] {
        match self {
            Self::Struct(layout) => &layout.fields,
            Self::Tuple(layout) => &layout.elements,
            Self::Object(layout) => &layout.fields,
            Self::None
            | Self::Scalar(_)
            | Self::Reference(_)
            | Self::FunctionPointer(_)
            | Self::Slice(_)
            | Self::Array(_)
            | Self::Vector(_)
            | Self::Tensor(_)
            | Self::TensorView(_)
            | Self::Variant(_)
            | Self::Dynamic
            | Self::Function(_)
            | Self::Newtype(_) => &[],
        }
    }
}

/// Concrete layout for one slice descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct SliceLayout {
    /// The backing data reference.
    pub data: ReferenceLayout,
}

/// Concrete layout for one reference value.
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
        if matches!(self.flags.kind(), Some(ReferenceKind::Raw)) {
            return Some(AddressSpace::Raw);
        }

        Some(match self.flags.storage()? {
            ReferenceStorage::Local => AddressSpace::Local,
            ReferenceStorage::Shared => AddressSpace::Shared,
            ReferenceStorage::Frame => AddressSpace::Frame,
            ReferenceStorage::Static => AddressSpace::Static,
        })
    }

    /// Return the native cell layout for this reference.
    pub fn cell_layout(&self) -> Option<CellLayout> {
        Some(self.address_space()?.cell_layout())
    }
}

/// Concrete layout for one closure value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FunctionLayout {
    /// The callable signature.
    pub signature: Signature,
    /// The captured environment type.
    pub environment: TypeId,
}

/// Concrete layout for a struct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct StructLayout {
    /// The fields in layout order.
    pub fields: Vec<LayoutField>,
}

/// Concrete layout for a tuple.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TupleLayout {
    /// The tuple elements in layout order.
    pub elements: Vec<LayoutField>,
}

/// Layout for inline indexed element storage.
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct VariantLayout {
    /// The tag layout.
    pub tag: VariantTagLayout,
    /// The variant payload byte offset.
    pub payload_offset: u32,
    /// The variant cases.
    pub variants: Vec<VariantCaseLayout>,
}

/// Concrete layout for a variant tag.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct VariantTagLayout {
    /// The tag type when it has been materialized.
    pub ty: Option<TypeId>,
    /// The tag size in bytes.
    pub size: u32,
    /// The tag alignment in bytes.
    pub alignment: u32,
}

/// Concrete layout for an object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ObjectLayout {
    /// Byte offset of the virtual dispatch pointer when this object carries one.
    pub dispatch_offset: Option<u32>,
    /// The fields in layout order.
    pub fields: Vec<LayoutField>,
}

/// Concrete layout for a nominal newtype.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct NewtypeLayout {
    /// The backing type.
    pub backing_type: TypeId,
    /// The backing type layout.
    pub backing_layout: LayoutId,
}

/// Memory layout for a single field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct LayoutField {
    /// Field name for lookup and debugging.
    pub name: Option<StringId>,
    /// Program type of the field.
    pub ty: TypeId,
    /// Byte offset from the start of the aggregate.
    pub offset: u32,
    /// Size of the field in bytes.
    pub size: u32,
    /// Alignment requirement of the field in bytes.
    pub alignment: u32,
    /// Original source index for stable mapping.
    pub source_index: Option<u32>,
}

/// Concrete layout for one variant case.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct VariantCaseLayout {
    /// The logical case type.
    pub ty: TypeId,
    /// The case layout.
    pub layout: LayoutId,
}
