use std::num::NonZeroU32;

use serde::{Deserialize, Serialize};

use destack_core::{FxIndexMap, SectionEntry, StringId};
use destack_serde::Reflect;

use crate::{FloatType, TraceMap, TypeId};

/// Canonical layout table for one MIR module.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Reflect)]
pub struct LayoutTable {
    /// Layout entries indexed by LayoutId.
    entries: Vec<Layout>,
    /// Layout ids keyed by type id.
    types: FxIndexMap<TypeId, LayoutId>,
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
    pub fn type_layout(&self, ty: TypeId) -> Option<&Layout> {
        let layout_id = self.layout_id(ty)?;

        Some(self.layout(layout_id))
    }

    /// Return the named field offsets of one laid-out struct type.
    pub fn named_field_offsets(&self, ty: TypeId) -> Vec<(StringId, u32)> {
        let layout = self.layout_id(ty).map(|id| self.layout(id));
        let mut fields = Vec::new();
        if let Some(LayoutShape::Struct(layout)) = layout.map(|layout| &layout.shape) {
            fields.extend(
                layout
                    .fields
                    .iter()
                    .filter_map(|field| field.name.map(|name| (name, field.offset))),
            );
        }

        fields
    }

    /// Return the layout id for a type when present.
    pub fn layout_id(&self, ty: TypeId) -> Option<LayoutId> {
        self.types.get(&ty).copied()
    }

    /// Iterate laid-out types and their layout ids.
    pub fn types(&self) -> impl Iterator<Item = (TypeId, LayoutId)> + '_ {
        self.types.iter().map(|(&ty, &layout)| (ty, layout))
    }

    /// Record the layout id for a type.
    pub fn set_layout_id(&mut self, ty: TypeId, layout_id: LayoutId) -> Option<LayoutId> {
        self.types.insert(ty, layout_id)
    }

    /// Copy structural layout table entries from one type id to another.
    pub fn copy_type_entries(&mut self, from: TypeId, to: TypeId) {
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

/// Concrete memory layout for one MIR type.
///
/// Initialized values store zero in every padding byte.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct Layout<T = TypeId, L = LayoutId> {
    /// The layout shape.
    pub shape: LayoutShape<T, L>,
    /// The physical value representation.
    pub representation: Representation,
    /// The largest scalar field containing invalid values.
    pub niche: Option<ScalarField>,
    /// Total size in bytes, including trailing padding.
    pub size: u32,
    /// Alignment requirement in bytes.
    pub alignment: u32,
    /// Reference trace map for this layout.
    pub trace_map: TraceMap,
    /// Whether no value of this layout exists.
    pub uninhabited: bool,
}

impl Layout {
    /// Create one scalar layout of the given size and alignment.
    pub const fn scalar(scalar: Scalar, size: u32, alignment: u32) -> Self {
        let niche = match scalar.validity.invalid(scalar.bit_width()) {
            Some(_) => Some(ScalarField::new(scalar, 0)),
            None => None,
        };

        Self {
            shape: LayoutShape::Scalar,
            representation: Representation::Scalar(scalar),
            niche,
            size,
            alignment,
            trace_map: TraceMap::Empty,
            uninhabited: false,
        }
    }
}

impl<T, L> Layout<T, L> {
    /// Return the byte width of this layout.
    pub const fn byte_len(&self) -> usize {
        self.size as usize
    }

    /// Return one field by layout index.
    pub fn field_at(&self, index: u32) -> Option<&LayoutField<T>> {
        self.shape.fields().get(index as usize)
    }

    /// Return one field by source index.
    pub fn source_field(&self, index: u32) -> Option<&LayoutField<T>> {
        self.shape
            .fields()
            .iter()
            .find(|field| field.source_index == index)
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

    /// Return the fixed element layout when present.
    pub const fn element(&self) -> Option<&ElementLayout<T>> {
        self.shape.elements()
    }

    /// Return the fixed element count when present.
    pub const fn element_count(&self) -> Option<usize> {
        match self.element() {
            Some(element) => Some(element.count as usize),
            None => None,
        }
    }

    /// Return whether this layout is a slice descriptor.
    pub const fn is_slice(&self) -> bool {
        matches!(self.shape, LayoutShape::Slice)
    }

    /// Return the aligned stride of this layout.
    pub fn stride(&self) -> usize {
        self.byte_len().next_multiple_of(self.alignment as usize)
    }
}

/// Physical representation of one MIR value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Representation {
    /// One scalar register value.
    Scalar(Scalar),
    /// Two scalar register values in canonical byte order.
    ScalarPair([ScalarField; 2]),
    /// One fixed vector value eligible for target legalization.
    Vector(Vector),
    /// Canonical bytes addressed in memory.
    Memory,
}

/// One scalar machine value and its valid bit range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Scalar {
    /// The scalar machine primitive.
    pub primitive: Primitive,
    /// The scalar values admitted by the type.
    pub validity: Validity,
}

impl Scalar {
    /// Create one scalar admitting every primitive bit pattern.
    pub const fn new(primitive: Primitive) -> Self {
        Self {
            validity: Validity::all(primitive.bit_width()),
            primitive,
        }
    }

    /// Create one scalar with an explicit valid bit range.
    pub const fn with_validity(primitive: Primitive, validity: Validity) -> Self {
        Self {
            primitive,
            validity,
        }
    }

    /// Create one reference scalar reserving the null and undefined words as niches.
    pub const fn reference(primitive: Primitive) -> Self {
        let maximum = scalar_mask(primitive.bit_width());

        Self::with_validity(primitive, Validity::new(2, maximum))
    }

    /// Return the scalar width in bits.
    pub const fn bit_width(self) -> u16 {
        self.primitive.bit_width()
    }

    /// Return the mask covering every scalar bit.
    pub const fn bit_mask(self) -> u128 {
        scalar_mask(self.bit_width())
    }

    /// Return this scalar as a niche field when it excludes any bit pattern.
    pub const fn niche(self, offset: u32) -> Option<ScalarField> {
        match self.validity.invalid(self.bit_width()) {
            Some(_) => Some(ScalarField::new(self, offset)),
            None => None,
        }
    }
}

/// One scalar primitive understood by physical backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub enum Primitive {
    /// An integer bit string.
    Integer { width: u16 },
    /// A floating point value.
    Float(FloatType),
    /// A process or program pointer.
    Pointer { width: u16 },
}

impl Primitive {
    /// Return the primitive width in bits.
    pub const fn bit_width(self) -> u16 {
        match self {
            Self::Integer { width } | Self::Pointer { width } => width,
            Self::Float(format) => format.width(),
        }
    }
}

/// One inclusive wrapping range of valid scalar bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Validity {
    /// The first valid scalar value.
    pub start: Discriminant,
    /// The last valid scalar value.
    pub end: Discriminant,
}

impl Validity {
    /// Create one inclusive wrapping valid range.
    pub const fn new(start: u128, end: u128) -> Self {
        Self {
            start: Discriminant::from_bits(start),
            end: Discriminant::from_bits(end),
        }
    }

    /// Admit every bit pattern of one scalar width.
    pub const fn all(width: u16) -> Self {
        Self::new(0, scalar_mask(width))
    }

    /// Return the contiguous wrapping range outside this validity range.
    pub const fn invalid(self, width: u16) -> Option<(Discriminant, u128)> {
        let mask = scalar_mask(width);
        let start = self.start.bits() & mask;
        let end = self.end.bits() & mask;
        let invalid_start = end.wrapping_add(1) & mask;
        if invalid_start == start {
            return None;
        }
        let invalid_end = start.wrapping_sub(1) & mask;
        let count = invalid_end.wrapping_sub(invalid_start) & mask;

        Some((Discriminant::from_bits(invalid_start), count + 1))
    }
}

/// One scalar field inside a pair representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct ScalarField {
    /// The scalar field representation.
    pub scalar: Scalar,
    /// The canonical byte offset.
    pub offset: u32,
}

impl ScalarField {
    /// Create one scalar field.
    pub const fn new(scalar: Scalar, offset: u32) -> Self {
        Self { scalar, offset }
    }

    /// Return the number of invalid scalar values available through this field.
    pub const fn invalid_count(self) -> u128 {
        match self.scalar.validity.invalid(self.scalar.bit_width()) {
            Some((_, count)) => count,
            None => 0,
        }
    }
}

/// One fixed vector representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Vector {
    /// The scalar lane representation.
    pub element: Scalar,
    /// The fixed lane count.
    pub lanes: u32,
}

impl Vector {
    /// Create one fixed vector representation.
    pub const fn new(element: Scalar, lanes: u32) -> Self {
        Self { element, lanes }
    }
}

/// Return the low-bit mask for one scalar width.
const fn scalar_mask(width: u16) -> u128 {
    if width >= u128::BITS as u16 {
        u128::MAX
    } else {
        (1u128 << width) - 1
    }
}

/// Concrete memory layout shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub enum LayoutShape<T = TypeId, L = LayoutId> {
    /// No runtime storage.
    None,
    /// Builtin scalar storage.
    Scalar,
    /// Struct storage.
    Struct(StructLayout<T>),
    /// Tuple storage.
    Tuple(TupleLayout<T>),
    /// Slice header storage.
    Slice,
    /// Fixed array storage.
    Array(ElementLayout<T>),
    /// Vector value storage.
    Vector(ElementLayout<T>),
    /// Variant value storage.
    Variant(VariantLayout<T>),
    /// Object field storage with optional virtual dispatch.
    Object(ObjectLayout<T>),
    /// Runtime dynamic value layout.
    Dynamic,
    /// Runtime function value storage.
    Function,
    /// Transparent nominal storage.
    Newtype(NewtypeLayout<T, L>),
}

impl<T, L> LayoutShape<T, L> {
    /// Return element layout when this shape stores indexed elements inline.
    pub const fn elements(&self) -> Option<&ElementLayout<T>> {
        match self {
            Self::Array(layout) | Self::Vector(layout) => Some(layout),
            _ => None,
        }
    }

    /// Return field layouts for field-addressable shapes.
    pub fn fields(&self) -> &[LayoutField<T>] {
        match self {
            Self::Struct(layout) => &layout.fields,
            Self::Tuple(layout) => &layout.elements,
            Self::Object(layout) => &layout.fields,
            Self::None
            | Self::Scalar
            | Self::Slice
            | Self::Array(_)
            | Self::Vector(_)
            | Self::Variant(_)
            | Self::Dynamic
            | Self::Function
            | Self::Newtype(_) => &[],
        }
    }

    /// Return this shape with field layouts attached when supported.
    pub fn with_fields(self, fields: Vec<LayoutField<T>>) -> Self {
        match self {
            Self::Struct(_) => Self::Struct(StructLayout { fields }),
            Self::Tuple(_) => Self::Tuple(TupleLayout { elements: fields }),
            Self::Object(layout) => Self::Object(ObjectLayout {
                dispatch_offset: layout.dispatch_offset,
                fields,
            }),
            Self::None
            | Self::Scalar
            | Self::Slice
            | Self::Array(_)
            | Self::Vector(_)
            | Self::Variant(_)
            | Self::Dynamic
            | Self::Function
            | Self::Newtype(_) => self,
        }
    }
}

/// Concrete layout for a struct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct StructLayout<T = TypeId> {
    /// The fields in source order.
    pub fields: Vec<LayoutField<T>>,
}

/// Concrete layout for a tuple.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct TupleLayout<T = TypeId> {
    /// The tuple elements in source order.
    pub elements: Vec<LayoutField<T>>,
}

/// Layout for inline indexed element storage.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ElementLayout<T = TypeId> {
    /// The stored element type.
    pub element: T,
    /// The byte stride between elements.
    pub stride: u32,
    /// The fixed element count.
    pub count: u32,
}

/// Concrete layout for a variant value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct VariantLayout<T = TypeId> {
    /// The logical discriminant type.
    pub discriminant: T,
    /// The physical discriminant encoding.
    pub encoding: VariantEncoding,
    /// The variant cases.
    pub cases: Vec<VariantCaseLayout<T>>,
}

/// Physical scalar field carrying a variant discriminant.
#[repr(C)]
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
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
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
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
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, SectionEntry,
)]
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
    pub const fn decode_niche(self, scalar: u128, case_count: u32) -> Option<u32> {
        let Self::Niche {
            field,
            untagged_case,
            niche_start,
        } = self
        else {
            return None;
        };
        if case_count == 0 || untagged_case >= case_count {
            return None;
        }

        let niche_count = case_count - 1;
        let value = field.extract(scalar);
        let relative = value.wrapping_sub(niche_start.bits()) & field.value_mask();
        if relative < niche_count as u128 {
            let case = relative as u32;
            let case = if case >= untagged_case {
                case + 1
            } else {
                case
            };

            Some(case)
        } else {
            Some(untagged_case)
        }
    }

    /// Encode one niche case into an existing physical scalar.
    pub const fn encode_niche(self, scalar: u128, case: u32, case_count: u32) -> Option<u128> {
        let Self::Niche {
            field,
            untagged_case,
            niche_start,
        } = self
        else {
            return None;
        };
        if case >= case_count || untagged_case >= case_count {
            return None;
        }

        if case == untagged_case {
            return Some(scalar);
        }

        let relative = if case > untagged_case { case - 1 } else { case };
        let relative = relative as u128;
        let value = niche_start.bits().wrapping_add(relative) & field.value_mask();

        Some(field.insert(scalar, value))
    }
}

/// Concrete layout for an object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct ObjectLayout<T = TypeId> {
    /// Byte offset of the virtual table id when present.
    pub dispatch_offset: Option<u32>,
    /// The fields in layout order.
    pub fields: Vec<LayoutField<T>>,
}

/// Concrete layout for a nominal newtype.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Reflect)]
pub struct NewtypeLayout<T = TypeId, L = LayoutId> {
    /// The backing type.
    pub backing_type: T,
    /// The backing type layout.
    pub backing_layout: L,
}

/// Memory layout for a single field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct LayoutField<T = TypeId> {
    /// Field name for lookup and debugging.
    pub name: Option<StringId>,
    /// MIR type of the field.
    pub ty: T,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Reflect)]
pub struct VariantCaseLayout<T = TypeId> {
    /// The logical discriminant bits.
    pub discriminant: Discriminant,
    /// The logical case type.
    pub ty: T,
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
            untagged_case: 1,
            niche_start: Discriminant::from_bits(254),
        };

        assert_eq!(encoding.decode_niche(254, 3), Some(0));
        assert_eq!(encoding.decode_niche(255, 3), Some(2));
        assert_eq!(encoding.decode_niche(1, 3), Some(1));
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
            untagged_case: 1,
            niche_start: Discriminant::from_bits(14),
        };
        let scalar = 0xA00B;

        assert_eq!(encoding.encode_niche(scalar, 0, 3), Some(0xA0EB));
        assert_eq!(encoding.encode_niche(scalar, 1, 3), Some(scalar));
        assert_eq!(encoding.encode_niche(scalar, 2, 3), Some(0xA0FB));
        assert_eq!(encoding.encode_niche(scalar, 3, 3), None);
    }
}
