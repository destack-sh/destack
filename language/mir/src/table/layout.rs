use std::collections::HashMap;
use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use destack_core::StringId;
use destack_serde::Reflect;

use crate::{LocalNodeId, TensorFormat, TensorSharding, TensorViewFormat, TraceMap, Type};

/// Canonical layout table for one MIR module.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub struct LayoutTable {
    /// Layout entries indexed by LayoutId.
    pub entries: Vec<Layout>,
    /// Layout ids keyed by type id.
    pub types: HashMap<LocalNodeId<Type>, LayoutId>,
}

impl LayoutTable {
    /// Create an empty layout table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a layout entry and return its id.
    pub fn insert(&mut self, layout: Layout) -> LayoutId {
        let next_index = self.entries.len() + 1;
        let id = LayoutId::new(next_index as u32);
        self.entries.push(layout);

        id
    }

    /// Return a layout entry for an id.
    pub fn layout(&self, id: LayoutId) -> &Layout {
        let index = id.index();
        self.entries
            .get(index)
            .unwrap_or_else(|| unreachable!("missing layout entry {index}"))
    }

    /// Return the layout entry for a type id when available.
    pub fn type_layout(&self, ty: LocalNodeId<Type>) -> Option<&Layout> {
        let layout_id = self.types.get(&ty)?;

        self.entries.get(layout_id.index())
    }

    /// Return the layout id for a type when present.
    pub fn layout_id(&self, ty: LocalNodeId<Type>) -> Option<LayoutId> {
        self.types.get(&ty).copied()
    }

    /// Record the layout id for a type.
    pub fn set_layout_id(
        &mut self,
        ty: LocalNodeId<Type>,
        layout_id: LayoutId,
    ) -> Option<LayoutId> {
        self.types.insert(ty, layout_id)
    }

    /// Copy structural layout table entries from one type id to another.
    pub fn copy_type_entries(&mut self, from: LocalNodeId<Type>, to: LocalNodeId<Type>) {
        if let Some(layout_id) = self.layout_id(from) {
            self.set_layout_id(to, layout_id);
        }
    }
}

/// Opaque identifier for a concrete memory layout.
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

    /// Return the raw layout identifier value.
    #[inline]
    pub const fn raw(self) -> u32 {
        self.0.get()
    }

    /// Return the zero based layout-table index.
    #[inline]
    pub const fn index(self) -> usize {
        self.raw() as usize - 1
    }
}

/// Concrete memory layout for an aggregate type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Layout {
    /// The layout shape.
    pub shape: LayoutShape,
    /// Total size in bytes, including trailing padding.
    pub size: u32,
    /// Alignment requirement in bytes.
    pub alignment: u32,
    /// Managed-reference trace map for this layout.
    pub trace_map: TraceMap,
}

impl Layout {
    /// Create one scalar layout of the given size and alignment.
    pub const fn scalar(size: u32, alignment: u32) -> Self {
        Self {
            shape: LayoutShape::Scalar,
            size,
            alignment,
            trace_map: TraceMap::Empty,
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

/// Concrete memory layout shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum LayoutShape {
    /// No runtime storage.
    None,
    /// Builtin scalar storage.
    Scalar,
    /// Struct storage.
    Struct(StructLayout),
    /// Tuple storage.
    Tuple(TupleLayout),
    /// Slice header storage.
    Slice,
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
    /// Object storage with a dispatch table header.
    Object(ObjectLayout),
    /// Runtime dynamic value layout.
    Dynamic,
    /// Runtime function value storage.
    Function,
    /// Transparent nominal storage.
    Newtype(NewtypeLayout),
}

impl LayoutShape {
    /// Return element layout when this shape stores indexed elements inline.
    pub const fn elements(&self) -> Option<&ElementLayout> {
        match self {
            Self::Array(layout) | Self::Vector(layout) => Some(layout),
            _ => None,
        }
    }

    /// Return field layouts for field-addressable shapes.
    pub fn fields(&self) -> &[LayoutField] {
        match self {
            Self::Struct(layout) => &layout.fields,
            Self::Tuple(layout) => &layout.elements,
            Self::Object(layout) => &layout.fields,
            Self::None
            | Self::Scalar
            | Self::Slice
            | Self::Array(_)
            | Self::Vector(_)
            | Self::Tensor(_)
            | Self::TensorView(_)
            | Self::Variant(_)
            | Self::Dynamic
            | Self::Function
            | Self::Newtype(_) => &[],
        }
    }

    /// Return this shape with field layouts attached when supported.
    pub fn with_fields(self, fields: Vec<LayoutField>) -> Self {
        match self {
            Self::Struct(_) => Self::Struct(StructLayout { fields }),
            Self::Tuple(_) => Self::Tuple(TupleLayout { elements: fields }),
            Self::Object(_) => Self::Object(ObjectLayout { fields }),
            Self::None
            | Self::Scalar
            | Self::Slice
            | Self::Array(_)
            | Self::Vector(_)
            | Self::Tensor(_)
            | Self::TensorView(_)
            | Self::Variant(_)
            | Self::Dynamic
            | Self::Function
            | Self::Newtype(_) => self,
        }
    }
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
    pub element: LocalNodeId<Type>,
    /// The byte stride between elements.
    pub stride: u32,
    /// The fixed element count.
    pub count: u32,
}

/// Concrete layout for a tensor handle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TensorLayout {
    /// The tensor element type.
    pub element: LocalNodeId<Type>,
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
    pub element: LocalNodeId<Type>,
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
    /// The logical discriminant type.
    pub discriminant: LocalNodeId<Type>,
    /// The logical payload storage type.
    pub storage: LocalNodeId<Type>,
    /// The physical discriminant encoding.
    pub encoding: VariantEncoding,
    /// The variant cases.
    pub cases: Vec<VariantCaseLayout>,
}

/// Physical scalar field carrying a variant discriminant.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct DiscriminantField {
    /// The byte offset from the variant base.
    pub offset: u32,
    /// The scalar storage width in bytes.
    pub byte_len: u8,
    /// The first discriminant bit inside the scalar.
    pub bit_offset: u8,
    /// The discriminant width in bits.
    pub bit_len: u8,
}

/// Target-independent logical variant discriminant bits.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub struct Discriminant {
    /// Low 64 bits.
    pub low: u64,
    /// High 64 bits.
    pub high: u64,
}

const _: () = assert!(std::mem::size_of::<Discriminant>() == 16);
const _: () = assert!(std::mem::align_of::<Discriminant>() == 8);

impl Discriminant {
    /// Create one discriminant from its logical bits.
    pub const fn from_bits(bits: u128) -> Self {
        Self {
            low: bits as u64,
            high: (bits >> u64::BITS) as u64,
        }
    }

    /// Return the logical discriminant bits.
    pub const fn bits(self) -> u128 {
        self.low as u128 | ((self.high as u128) << u64::BITS)
    }
}

impl From<u128> for Discriminant {
    fn from(bits: u128) -> Self {
        Self::from_bits(bits)
    }
}

impl From<Discriminant> for u128 {
    fn from(discriminant: Discriminant) -> Self {
        discriminant.bits()
    }
}

impl DiscriminantField {
    /// Create one full-width scalar discriminant field.
    pub const fn scalar(offset: u32, byte_len: u8) -> Self {
        Self {
            offset,
            byte_len,
            bit_offset: 0,
            bit_len: byte_len * 8,
        }
    }

    /// Return the unshifted discriminant mask.
    pub const fn mask(self) -> u128 {
        let bits = if self.bit_len == u128::BITS as u8 {
            u128::MAX
        } else {
            (1u128 << self.bit_len) - 1
        };

        bits << self.bit_offset
    }

    /// Extract the discriminant field from one scalar value.
    pub const fn extract(self, scalar: u128) -> u128 {
        (scalar & self.mask()) >> self.bit_offset
    }

    /// Insert one discriminant field into an existing scalar value.
    pub const fn insert(self, scalar: u128, discriminant: u128) -> u128 {
        let mask = self.mask();
        let discriminant = (discriminant << self.bit_offset) & mask;

        (scalar & !mask) | discriminant
    }

    /// Return the wrapping mask for extracted discriminant values.
    pub const fn value_mask(self) -> u128 {
        if self.bit_len == u128::BITS as u8 {
            u128::MAX
        } else {
            (1u128 << self.bit_len) - 1
        }
    }
}

/// Physical encoding for one variant discriminant.
#[repr(C, u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum VariantEncoding {
    /// Store the logical discriminant directly.
    Direct {
        /// The physical discriminant field.
        field: DiscriminantField,
    },
    /// Encode selected cases in invalid values of one payload field.
    Niche {
        /// The payload field carrying the niche.
        field: DiscriminantField,
        /// The case represented by every value outside the niche range.
        untagged_case: u32,
        /// The first case represented in the niche range.
        niche_case_start: u32,
        /// The last case represented in the niche range.
        niche_case_end: u32,
        /// The first physical niche value.
        niche_start: Discriminant,
    },
}

const _: () = assert!(std::mem::size_of::<VariantEncoding>() <= 48);

impl VariantEncoding {
    /// Return the physical discriminant field.
    pub const fn field(self) -> DiscriminantField {
        match self {
            Self::Direct { field } | Self::Niche { field, .. } => field,
        }
    }

    /// Decode one physical scalar into a zero-based case index when niche encoded.
    pub const fn decode_niche(self, scalar: u128) -> Option<u32> {
        let Self::Niche {
            field,
            untagged_case,
            niche_case_start,
            niche_case_end,
            niche_start,
        } = self
        else {
            return None;
        };
        let value = field.extract(scalar);
        let relative = value.wrapping_sub(niche_start.bits()) & field.value_mask();
        let niche_count = niche_case_end - niche_case_start;

        if relative <= niche_count as u128 {
            Some(niche_case_start + relative as u32)
        } else {
            Some(untagged_case)
        }
    }

    /// Encode one niche case into an existing physical scalar.
    pub const fn encode_niche(self, scalar: u128, case: u32) -> Option<u128> {
        let Self::Niche {
            field,
            untagged_case,
            niche_case_start,
            niche_case_end,
            niche_start,
        } = self
        else {
            return None;
        };

        if case == untagged_case {
            return Some(scalar);
        }
        if case < niche_case_start || case > niche_case_end {
            return None;
        }

        let relative = (case - niche_case_start) as u128;
        let value = niche_start.bits().wrapping_add(relative) & field.value_mask();

        Some(field.insert(scalar, value))
    }
}

/// Concrete layout for an object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ObjectLayout {
    /// The fields in layout order.
    pub fields: Vec<LayoutField>,
}

/// Concrete layout for a nominal newtype.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct NewtypeLayout {
    /// The backing type.
    pub backing_type: LocalNodeId<Type>,
    /// The backing type layout.
    pub backing_layout: LayoutId,
}

/// Memory layout for a single field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct LayoutField {
    /// Field name for lookup and debugging.
    pub name: Option<StringId>,
    /// MIR type of the field.
    pub ty: LocalNodeId<Type>,
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
    /// The logical discriminant bits.
    pub discriminant: Discriminant,
    /// The logical case type.
    pub ty: LocalNodeId<Type>,
    /// The payload byte offset from the variant base.
    pub payload_offset: u32,
}

#[cfg(test)]
mod tests {
    use super::{Discriminant, DiscriminantField, VariantEncoding};

    /// Preserve unrelated scalar bits when inserting one discriminant field.
    #[test]
    fn test_insert_discriminant_field() {
        let field = DiscriminantField {
            offset: 4,
            byte_len: 2,
            bit_offset: 4,
            bit_len: 3,
        };
        let scalar = 0b1010_0001;
        let encoded = field.insert(scalar, 0b011);

        assert_eq!(encoded, 0b1011_0001);
        assert_eq!(field.extract(encoded), 0b011);
    }

    /// Decode niche cases and the untagged payload case.
    #[test]
    fn test_decode_niche_variant() {
        let encoding = VariantEncoding::Niche {
            field: DiscriminantField::scalar(0, 1),
            untagged_case: 0,
            niche_case_start: 1,
            niche_case_end: 2,
            niche_start: Discriminant::from_bits(254),
        };

        assert_eq!(encoding.decode_niche(254), Some(1));
        assert_eq!(encoding.decode_niche(255), Some(2));
        assert_eq!(encoding.decode_niche(1), Some(0));
    }

    /// Encode niche cases without disturbing adjacent payload bits.
    #[test]
    fn test_encode_niche_variant() {
        let encoding = VariantEncoding::Niche {
            field: DiscriminantField {
                offset: 0,
                byte_len: 2,
                bit_offset: 4,
                bit_len: 4,
            },
            untagged_case: 0,
            niche_case_start: 1,
            niche_case_end: 2,
            niche_start: Discriminant::from_bits(14),
        };
        let scalar = 0xA00B;

        assert_eq!(encoding.encode_niche(scalar, 1), Some(0xA0EB));
        assert_eq!(encoding.encode_niche(scalar, 2), Some(0xA0FB));
        assert_eq!(encoding.encode_niche(scalar, 3), None);
    }
}
