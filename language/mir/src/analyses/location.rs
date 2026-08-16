use std::collections::HashMap;

use crate as mir;

use crate::{AliasResult, TargetLayout};

use super::DefinitionTable;

/// One memory location reached through an address.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    /// The address-bearing value being dereferenced.
    pub address: mir::Value,
    /// Size of the access in bytes, if known.
    pub size: Option<u64>,
    /// The value type being accessed, when known.
    pub value_type: Option<mir::TypeId>,
    /// Reference kind for the address, when known.
    pub reference_kind: Option<mir::ReferenceKind>,
    /// Storage for the address, when known.
    pub reference_storage: Option<mir::Storage>,
}

impl MemoryLocation {
    /// Create a location from an address with unknown size.
    pub fn from_address(address: mir::Value) -> Self {
        Self {
            address,
            size: None,
            value_type: None,
            reference_kind: None,
            reference_storage: None,
        }
    }

    /// Create a location with known size.
    pub fn with_size(address: mir::Value, size: u64) -> Self {
        Self {
            address,
            size: Some(size),
            value_type: None,
            reference_kind: None,
            reference_storage: None,
        }
    }

    /// Create a fully specified location.
    pub fn new(
        address: mir::Value,
        size: Option<u64>,
        value_type: Option<mir::TypeId>,
        reference_kind: Option<mir::ReferenceKind>,
        reference_storage: Option<mir::Storage>,
    ) -> Self {
        Self {
            address,
            size,
            value_type,
            reference_kind,
            reference_storage,
        }
    }

    /// Return the memory spaces this location can touch.
    pub fn spaces(&self) -> mir::StorageSet {
        self.reference_storage
            .as_ref()
            .map(|storage| storage.storage_set())
            .unwrap_or(mir::StorageSet::ANY)
    }

    /// Return aliasing for another location with the same address value.
    pub fn alias_same_address(&self, other: &MemoryLocation) -> AliasResult {
        match (self.size, other.size) {
            (Some(left), Some(right)) if left == right => AliasResult::MustAlias,
            (Some(_), Some(_)) => AliasResult::PartialAlias,
            _ => AliasResult::PartialAlias,
        }
    }

    /// Return whether both locations are compatible for value forwarding.
    pub fn is_compatible_with(&self, other: &MemoryLocation) -> bool {
        // compare byte sizes when both sides know them
        if let (Some(left_size), Some(right_size)) = (self.size, other.size)
            && left_size != right_size
        {
            return false;
        }

        // compare value types when both sides know them
        if let (Some(left_type), Some(right_type)) = (&self.value_type, &other.value_type)
            && left_type != right_type
        {
            return false;
        }

        true
    }
}

/// Memory region touched by one address-bearing value or direct access.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MemoryRegion {
    /// Memory access through an address-bearing value.
    Address {
        /// The addressed location.
        location: MemoryLocation,
        /// The memory spaces the address may touch.
        spaces: mir::StorageSet,
    },
    /// A precise memory place.
    Place(MemoryPlace),
    /// Local slot access.
    Local(mir::LocalNodeId<mir::Local>),
    /// Any memory in the given spaces.
    Any {
        /// The memory spaces that may be touched.
        spaces: mir::StorageSet,
    },
}

impl MemoryRegion {
    /// Create an imprecise region for all memory spaces.
    pub fn any() -> Self {
        Self::Any {
            spaces: mir::StorageSet::ANY,
        }
    }

    /// Create an imprecise region for a set of memory spaces.
    pub fn any_spaces(spaces: mir::StorageSet) -> Self {
        Self::Any { spaces }
    }

    /// Create an imprecise region for one storage region.
    pub fn any_storage(storage: mir::Storage) -> Self {
        Self::Any {
            spaces: storage.storage_set(),
        }
    }

    /// Return whether this region is owned by the current activation frame.
    pub fn is_frame_storage(&self) -> bool {
        matches!(self, Self::Local(_))
            || matches!(self, Self::Place(place) if matches!(place.root, StorageRoot::LocalSlot(_)))
    }

    /// Create an address access with an optional value type and inferred size.
    pub fn from_address(
        address: mir::Value,
        value_type: Option<mir::TypeId>,
        reference_kind: Option<mir::ReferenceKind>,
        reference_storage: Option<mir::Storage>,
        pointer_width_bits: u16,
        tree: &mir::Tree,
    ) -> Self {
        Self::from_address_with_size(
            address,
            value_type,
            reference_kind,
            reference_storage,
            None,
            pointer_width_bits,
            tree,
        )
    }

    /// Create an address access with an explicit size override.
    pub fn from_address_with_size(
        address: mir::Value,
        value_type: Option<mir::TypeId>,
        reference_kind: Option<mir::ReferenceKind>,
        reference_storage: Option<mir::Storage>,
        size: Option<u64>,
        pointer_width_bits: u16,
        tree: &mir::Tree,
    ) -> Self {
        let inferred_size = size.or_else(|| {
            value_type
                .as_ref()
                .and_then(|value_type| tree.get(*value_type).byte_size(tree, pointer_width_bits))
        });

        Self::Address {
            location: MemoryLocation::new(
                address,
                inferred_size,
                value_type,
                reference_kind,
                reference_storage,
            ),
            spaces: mir::StorageSet::ANY,
        }
    }

    /// Return the addressed location when this region has one.
    pub fn location(&self) -> Option<&MemoryLocation> {
        match self {
            Self::Address { location, .. } => Some(location),
            _ => None,
        }
    }

    /// Return the memory spaces covered by this region.
    pub fn spaces(&self) -> mir::StorageSet {
        match self {
            MemoryRegion::Address { spaces, .. } => *spaces,
            MemoryRegion::Place(place) => place.root.spaces(),
            MemoryRegion::Local(_) => mir::StorageSet::FRAME,
            MemoryRegion::Any { spaces } => *spaces,
        }
    }

    /// Set spaces on imprecise or addressed regions.
    pub fn set_spaces(&mut self, new_spaces: mir::StorageSet) {
        match self {
            Self::Address { spaces, .. } | Self::Any { spaces } => {
                *spaces = new_spaces;
            }
            Self::Place(_) | Self::Local(_) => {}
        }
    }

    /// Return whether this region may touch one storage root.
    pub fn may_touch_root(&self, storage: &StorageRoot) -> bool {
        match self {
            MemoryRegion::Place(place) => !place.root.is_disjoint_from(storage),
            MemoryRegion::Local(local) => {
                !storage.is_disjoint_from(&StorageRoot::LocalSlot(*local))
            }
            MemoryRegion::Any { spaces } => !spaces.is_disjoint(storage.spaces()),
            MemoryRegion::Address { spaces, .. } => !spaces.is_disjoint(storage.spaces()),
        }
    }
}

/// Identified storage root for one memory place.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StorageRoot {
    /// Local slot address.
    LocalSlot(mir::LocalNodeId<mir::Local>),
    /// Global storage address.
    Global {
        /// The global declaration.
        global: mir::LocalNodeId<mir::Global>,
        /// The global storage class.
        storage: mir::GlobalStorage,
    },
    /// Heap allocation instruction.
    Allocation {
        /// The allocation instruction.
        instruction: mir::LocalNodeId<mir::Instruction>,
        /// The allocation storage space.
        space: mir::Space,
        /// The reference kind produced by the allocation.
        kind: mir::ReferenceKind,
    },
    /// Function parameter.
    Parameter {
        /// Parameter index.
        index: u32,
        /// The parameter storage.
        storage: mir::Storage,
        /// The parameter reference kind.
        kind: mir::ReferenceKind,
        /// The parameter access.
        access: mir::Access,
    },
}

impl StorageRoot {
    /// Return true if this parameter is exclusive.
    pub fn is_exclusive_parameter(&self) -> bool {
        matches!(
            self,
            StorageRoot::Parameter {
                access: mir::Access::Exclusive,
                ..
            }
        )
    }

    /// Return whether two identified storage roots are disjoint.
    pub fn is_disjoint_from(&self, other: &StorageRoot) -> bool {
        match (self, other) {
            (StorageRoot::LocalSlot(left), StorageRoot::LocalSlot(right)) => left != right,
            (
                StorageRoot::Global { global: left, .. },
                StorageRoot::Global { global: right, .. },
            ) => left != right,
            (
                StorageRoot::Allocation {
                    instruction: left, ..
                },
                StorageRoot::Allocation {
                    instruction: right, ..
                },
            ) => left != right,
            (StorageRoot::Parameter { .. }, _) | (_, StorageRoot::Parameter { .. }) => false,
            _ => true,
        }
    }

    /// Return whether exclusive parameter constraints prove disjointness.
    pub fn exclusive_parameters_are_disjoint(&self, other: &StorageRoot) -> bool {
        match (self, other) {
            (
                StorageRoot::Parameter {
                    access: mir::Access::Exclusive,
                    ..
                },
                StorageRoot::Parameter {
                    access: mir::Access::Exclusive,
                    ..
                },
            ) => self != other,
            _ => false,
        }
    }

    /// Return the memory spaces covered by this storage root.
    pub fn spaces(&self) -> mir::StorageSet {
        match self {
            StorageRoot::LocalSlot(_) => mir::StorageSet::FRAME,
            StorageRoot::Global { storage, .. } => storage.storage_set(),
            StorageRoot::Allocation { space, .. } => space.space_set(),
            StorageRoot::Parameter { storage, .. } => storage.storage_set(),
        }
    }
}

/// Indexed byte offset component in address arithmetic.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IndexedOffset {
    /// The index value.
    pub index: mir::Value,
    /// Scale factor (element size in bytes).
    pub scale: u64,
}

/// Precise memory place representation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryPlace {
    /// The identified storage root.
    pub root: StorageRoot,
    /// Constant byte offset from storage root.
    pub const_offset: i64,
    /// Indexed offsets with their scales.
    pub indexed_offsets: Vec<IndexedOffset>,
    /// Field path from storage root.
    pub fields: Vec<u32>,
}

impl MemoryPlace {
    /// Create a memory place from one storage root.
    pub fn from_root(root: StorageRoot) -> Self {
        Self {
            root,
            const_offset: 0,
            fields: Vec::new(),
            indexed_offsets: Vec::new(),
        }
    }

    /// Return whether this place has only constant offsets.
    pub fn is_constant_offset(&self) -> bool {
        self.indexed_offsets.is_empty()
    }

    /// Add a constant offset.
    pub fn add_const_offset(&mut self, offset: i64) {
        self.const_offset = self.const_offset.saturating_add(offset);
    }

    /// Add a field index to the path.
    pub fn add_field(&mut self, field_index: u32) {
        self.fields.push(field_index);
    }

    /// Add an indexed offset.
    pub fn add_indexed_offset(&mut self, index: mir::Value, scale: u64) {
        self.indexed_offsets.push(IndexedOffset { index, scale });
    }

    /// Return aliasing with another place under known access locations.
    pub fn alias_with(
        &self,
        location: &MemoryLocation,
        other: &MemoryPlace,
        other_location: &MemoryLocation,
    ) -> AliasResult {
        // disjoint field paths cannot alias
        if self.fields_are_disjoint_from(other) {
            return AliasResult::NoAlias;
        }

        // constant byte ranges can be compared exactly
        if self.is_constant_offset()
            && other.is_constant_offset()
            && let (Some(size), Some(other_size)) = (location.size, other_location.size)
        {
            let range = ByteRange::new(self.const_offset, size);
            let other_range = ByteRange::new(other.const_offset, other_size);

            return match range.relation(other_range) {
                RangeRelation::Disjoint => AliasResult::NoAlias,
                RangeRelation::Equal => AliasResult::MustAlias,
                _ => AliasResult::PartialAlias,
            };
        }

        // identical indexed offset values can still prove disjoint byte ranges
        if self.indexed_offsets_are_disjoint_from(other, location.size, other_location.size) {
            return AliasResult::NoAlias;
        }

        AliasResult::MayAlias
    }

    /// Return whether two field paths are statically disjoint.
    fn fields_are_disjoint_from(&self, other: &MemoryPlace) -> bool {
        if self.fields.is_empty() || other.fields.is_empty() {
            return false;
        }

        self.fields
            .iter()
            .zip(other.fields.iter())
            .any(|(left, right)| left != right)
    }

    /// Return whether two indexed ranges are statically disjoint.
    fn indexed_offsets_are_disjoint_from(
        &self,
        other: &MemoryPlace,
        size: Option<u64>,
        other_size: Option<u64>,
    ) -> bool {
        if self.indexed_offsets.len() != 1 || other.indexed_offsets.len() != 1 {
            return false;
        }

        let offset = &self.indexed_offsets[0];
        let other_offset = &other.indexed_offsets[0];
        if offset.scale != other_offset.scale {
            return false;
        }

        let (Some(size), Some(other_size)) = (
            size.or(Some(offset.scale)),
            other_size.or(Some(other_offset.scale)),
        ) else {
            return false;
        };

        if offset.index != other_offset.index {
            return false;
        }

        let range = ByteRange::new(self.const_offset, size);
        let other_range = ByteRange::new(other.const_offset, other_size);
        let relation = range.relation(other_range);

        matches!(relation, RangeRelation::Disjoint)
    }
}

/// Builder for resolving memory regions by walking the def chain.
#[derive(Debug)]
pub(super) struct MemoryRegionBuilder<'a> {
    /// Cached region results.
    cache: HashMap<mir::Value, MemoryRegion>,
    /// Value definitions for address provenance.
    definitions: &'a DefinitionTable,
    /// The MIR tree.
    tree: &'a mir::Tree,
    /// The MIR function.
    function: &'a mir::Function,
    /// Type context for layout sensitive operations.
    target_layout: TargetLayout,
}

impl<'a> MemoryRegionBuilder<'a> {
    /// Create a new region builder.
    pub(super) fn new(
        function: &'a mir::Function,
        definitions: &'a DefinitionTable,
        tree: &'a mir::Tree,
        target_layout: TargetLayout,
    ) -> Self {
        Self {
            cache: HashMap::new(),
            definitions,
            tree,
            function,
            target_layout,
        }
    }

    /// Resolve an address-bearing value to a memory region.
    pub(super) fn region(&mut self, address: mir::Value) -> MemoryRegion {
        // check cache
        if let Some(cached) = self.cache.get(&address) {
            return cached.clone();
        }

        let result = self.region_impl(address);
        self.cache.insert(address, result.clone());
        result
    }

    /// Resolve an address-bearing value to a memory region.
    fn region_impl(&mut self, address: mir::Value) -> MemoryRegion {
        // check if it's a parameter
        for (index, parameter) in self.function.parameters.iter().enumerate() {
            if parameter.value == address {
                return self.parameter_region(index, parameter);
            }
        }

        // check if it's defined by an instruction
        let Some(instruction_id) = self.definitions.instruction(address) else {
            return self.any_region(address);
        };

        let instruction = self.tree.get(instruction_id);

        match instruction {
            mir::Instruction::NewZeroed { destination, .. }
            | mir::Instruction::NewUninit { destination, .. }
                if *destination == address =>
            {
                self.allocation_region(instruction_id, address)
            }
            mir::Instruction::NewSliceZeroed { destination, .. }
            | mir::Instruction::NewSliceUninit { destination, .. }
                if *destination == address =>
            {
                self.allocation_region(instruction_id, address)
            }
            mir::Instruction::NewComplete {
                destination, value, ..
            } if *destination == address => {
                let value = *value;

                self.region(value)
            }

            // identify global storage
            mir::Instruction::GlobalAddr {
                destination,
                global,
                ..
            } if *destination == address => {
                let global_id = *global;
                let global = self.tree.get(global_id);

                MemoryRegion::Place(MemoryPlace::from_root(StorageRoot::Global {
                    global: global_id,
                    storage: global.storage,
                }))
            }
            mir::Instruction::LocalAddr {
                destination, local, ..
            } if *destination == address => {
                MemoryRegion::Place(MemoryPlace::from_root(StorageRoot::LocalSlot(*local)))
            }

            // extend precise region with a field path
            mir::Instruction::FieldAddr {
                destination,
                aggregate,
                field,
                ..
            } if *destination == address => {
                let aggregate = *aggregate;

                let mut region = self.region(aggregate);
                if let MemoryRegion::Place(place) = &mut region {
                    place.add_field(*field);
                }

                region
            }

            // extend precise region with an indexed offset
            mir::Instruction::ElementAddr {
                destination,
                base,
                index,
                ..
            } if *destination == address => {
                let base = *base;
                let index = *index;

                let mut region = self.region(base);

                let scale = self.expect_element_size(base);
                if let MemoryRegion::Place(place) = &mut region {
                    place.add_indexed_offset(index, scale);
                }

                region
            }

            // preserve variant storage provenance without claiming disjoint payloads
            mir::Instruction::VariantPayloadAddr {
                destination,
                variant,
                ..
            } if *destination == address => self.region(*variant),

            // casts preserve provenance
            mir::Instruction::Cast {
                destination,
                argument,
                ..
            } if *destination == address => {
                let argument = *argument;

                self.region(argument)
            }

            // anything else is imprecise
            _ => self.any_region(address),
        }
    }

    /// Return a region for a parameter address.
    fn parameter_region(&self, index: usize, parameter: &mir::FunctionParameter) -> MemoryRegion {
        let ty = self.tree.get(parameter.ty);

        match (
            ty.reference_kind(),
            ty.reference_storage(),
            ty.reference_access(),
        ) {
            (Some(kind), Some(storage), Some(access)) => {
                MemoryRegion::Place(MemoryPlace::from_root(StorageRoot::Parameter {
                    index: index as u32,
                    storage,
                    kind,
                    access,
                }))
            }
            _ => MemoryRegion::any(),
        }
    }

    /// Return a region for a heap allocation.
    fn allocation_region(
        &self,
        instruction: mir::LocalNodeId<mir::Instruction>,
        address: mir::Value,
    ) -> MemoryRegion {
        let ty_id = self.value_type(address);
        let ty = self.tree.get(ty_id);

        match (ty.reference_kind(), ty.reference_storage()) {
            (Some(kind), Some(storage)) => {
                let mir::Storage::Heap(space) = storage else {
                    return MemoryRegion::any_storage(storage);
                };

                MemoryRegion::Place(MemoryPlace::from_root(StorageRoot::Allocation {
                    instruction,
                    space,
                    kind,
                }))
            }
            _ => MemoryRegion::any(),
        }
    }

    /// Return an imprecise region bounded by an address type when possible.
    fn any_region(&self, address: mir::Value) -> MemoryRegion {
        let ty_id = self.value_type(address);
        let ty = self.tree.get(ty_id);

        ty.reference_storage()
            .map_or_else(MemoryRegion::any, MemoryRegion::any_storage)
    }

    /// Return the value type for an SSA value.
    fn value_type(&self, value: mir::Value) -> mir::LocalNodeId<mir::Type> {
        self.function.expect_value_type(value)
    }

    /// Return the byte stride for one indexed value.
    fn expect_element_size(&self, array: mir::Value) -> u64 {
        let ty_id = self.value_type(array);
        let element_id = match self.tree.get(ty_id) {
            mir::Type::FixedArray { element, .. }
            | mir::Type::Slice { element, .. }
            | mir::Type::Tensor { element, .. }
            | mir::Type::TensorView { element, .. } => *element,
            mir::Type::Reference { pointee, .. } | mir::Type::Pointer { pointee, .. } => {
                self.expect_pointee_element(*pointee)
            }
            _ => panic!("element.address requires an indexed value, got {ty_id:?}"),
        };

        match self
            .tree
            .get(element_id)
            .byte_size(self.tree, self.target_layout.pointer_bits())
        {
            Some(size) if size > 0 => size,
            _ => panic!("element.address requires a byte-sized element, got {element_id:?}"),
        }
    }

    /// Return the element type for an indexed pointee.
    fn expect_pointee_element(&self, pointee: mir::TypeId) -> mir::TypeId {
        match self.tree.get(pointee) {
            mir::Type::FixedArray { element, .. }
            | mir::Type::Slice { element, .. }
            | mir::Type::Tensor { element, .. }
            | mir::Type::TensorView { element, .. } => *element,
            _ => panic!("element.address requires an indexed pointee, got {pointee:?}"),
        }
    }
}

/// One half-open byte range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ByteRange {
    /// Start offset in bytes.
    pub offset: i64,
    /// Size in bytes.
    pub size: u64,
}

impl ByteRange {
    /// Create a byte range.
    pub fn new(offset: i64, size: u64) -> Self {
        Self { offset, size }
    }

    /// Return the exclusive end offset.
    pub fn end(self) -> i64 {
        self.offset.saturating_add(self.size as i64)
    }

    /// Return whether this range overlaps another range.
    pub fn overlaps(self, other: ByteRange) -> bool {
        let end = self.end();
        let other_end = other.end();

        !(end <= other.offset || other_end <= self.offset)
    }

    /// Return whether this range exactly equals another range.
    pub fn equals(self, other: ByteRange) -> bool {
        self.offset == other.offset && self.size == other.size
    }

    /// Return the relationship between this range and another range.
    pub fn relation(self, other: ByteRange) -> RangeRelation {
        let end = self.end();
        let other_end = other.end();

        // disjoint
        if end <= other.offset || other_end <= self.offset {
            return RangeRelation::Disjoint;
        }

        // equal
        if self.equals(other) {
            return RangeRelation::Equal;
        }

        // containment
        if self.offset <= other.offset && end >= other_end {
            return RangeRelation::Contains;
        }
        if other.offset <= self.offset && other_end >= end {
            return RangeRelation::ContainedBy;
        }

        RangeRelation::Overlaps
    }
}

/// Relationship between two byte ranges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeRelation {
    /// Ranges are disjoint.
    Disjoint,
    /// Ranges are exactly equal.
    Equal,
    /// First range contains second.
    Contains,
    /// Second range contains first.
    ContainedBy,
    /// Ranges partially overlap.
    Overlaps,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ranges that overlap are detected as overlapping.
    #[test]
    fn test_ranges_overlap() {
        assert!(!ByteRange::new(0, 4).overlaps(ByteRange::new(10, 4)));
        assert!(!ByteRange::new(10, 4).overlaps(ByteRange::new(0, 4)));

        assert!(!ByteRange::new(0, 4).overlaps(ByteRange::new(4, 4)));

        assert!(ByteRange::new(0, 8).overlaps(ByteRange::new(4, 8)));
        assert!(ByteRange::new(4, 8).overlaps(ByteRange::new(0, 8)));

        assert!(ByteRange::new(0, 16).overlaps(ByteRange::new(4, 4)));
        assert!(ByteRange::new(4, 4).overlaps(ByteRange::new(0, 16)));

        assert!(ByteRange::new(0, 8).overlaps(ByteRange::new(0, 8)));
    }

    /// Range relationships return the expected classification.
    #[test]
    fn test_range_relation() {
        assert_eq!(
            ByteRange::new(0, 4).relation(ByteRange::new(10, 4)),
            RangeRelation::Disjoint
        );
        assert_eq!(
            ByteRange::new(0, 8).relation(ByteRange::new(0, 8)),
            RangeRelation::Equal
        );
        assert_eq!(
            ByteRange::new(0, 16).relation(ByteRange::new(4, 4)),
            RangeRelation::Contains
        );
        assert_eq!(
            ByteRange::new(4, 4).relation(ByteRange::new(0, 16)),
            RangeRelation::ContainedBy
        );
        assert_eq!(
            ByteRange::new(0, 8).relation(ByteRange::new(4, 8)),
            RangeRelation::Overlaps
        );
    }

    /// Storage roots expose their memory spaces.
    #[test]
    fn test_storage_spaces() {
        let local = StorageRoot::LocalSlot(mir::LocalNodeId::new(0));
        let allocation = StorageRoot::Allocation {
            instruction: mir::LocalNodeId::new(1),
            space: mir::Space::Shared,
            kind: mir::ReferenceKind::Managed,
        };
        let parameter = StorageRoot::Parameter {
            index: 0,
            storage: mir::Storage::Heap(mir::Space::Local),
            kind: mir::ReferenceKind::Borrowed,
            access: mir::Access::Mutable,
        };

        assert_eq!(local.spaces(), mir::StorageSet::FRAME);
        assert_eq!(allocation.spaces(), mir::StorageSet::SHARED);
        assert_eq!(parameter.spaces(), mir::StorageSet::LOCAL);
    }

    /// Memory places track constant and indexed offsets.
    #[test]
    fn test_memory_place_const_offset() {
        let mut place = MemoryPlace::from_root(StorageRoot::LocalSlot(mir::LocalNodeId::new(0)));
        assert!(place.is_constant_offset());

        place.add_const_offset(16);
        assert!(place.is_constant_offset());
        assert_eq!(place.const_offset, 16);

        place.add_indexed_offset(mir::Value::new(0), 4);
        assert!(!place.is_constant_offset());
    }

    /// Exclusive parameters are reported as exclusive.
    #[test]
    fn test_storage_is_exclusive_parameter() {
        let exclusive_parameter = StorageRoot::Parameter {
            index: 0,
            storage: mir::Storage::Heap(mir::Space::Local),
            kind: mir::ReferenceKind::Borrowed,
            access: mir::Access::Exclusive,
        };
        let mutable_parameter = StorageRoot::Parameter {
            index: 1,
            storage: mir::Storage::Heap(mir::Space::Local),
            kind: mir::ReferenceKind::Borrowed,
            access: mir::Access::Mutable,
        };
        let local = StorageRoot::LocalSlot(mir::LocalNodeId::new(0));

        assert!(exclusive_parameter.is_exclusive_parameter());
        assert!(!mutable_parameter.is_exclusive_parameter());
        assert!(!local.is_exclusive_parameter());
    }

    /// Field paths are captured by memory places.
    #[test]
    fn test_memory_place_fields() {
        let mut place = MemoryPlace::from_root(StorageRoot::LocalSlot(mir::LocalNodeId::new(0)));
        assert!(place.fields.is_empty());

        place.add_field(0);
        assert_eq!(place.fields, vec![0]);

        place.add_field(2);
        assert_eq!(place.fields, vec![0, 2]);

        assert!(place.is_constant_offset());
    }

    /// Multiple indexed offsets are tracked.
    #[test]
    fn test_memory_place_multiple_indexed_offsets() {
        let mut place = MemoryPlace::from_root(StorageRoot::Allocation {
            instruction: mir::LocalNodeId::new(0),
            space: mir::Space::Local,
            kind: mir::ReferenceKind::Unique,
        });

        place.add_indexed_offset(mir::Value::new(1), 4);
        place.add_indexed_offset(mir::Value::new(2), 8);

        assert!(!place.is_constant_offset());
        assert_eq!(place.indexed_offsets.len(), 2);
        assert_eq!(place.indexed_offsets[0].index, mir::Value::new(1));
        assert_eq!(place.indexed_offsets[0].scale, 4);
        assert_eq!(place.indexed_offsets[1].index, mir::Value::new(2));
        assert_eq!(place.indexed_offsets[1].scale, 8);
    }

    /// Constant offsets accumulate.
    #[test]
    fn test_memory_place_const_offset_accumulation() {
        let mut place = MemoryPlace::from_root(StorageRoot::Global {
            global: mir::LocalNodeId::new(0),
            storage: mir::GlobalStorage::Constant,
        });

        place.add_const_offset(8);
        place.add_const_offset(16);

        assert_eq!(place.const_offset, 24);
    }

    /// Negative offsets are handled consistently.
    #[test]
    fn test_memory_place_negative_offset() {
        let mut place = MemoryPlace::from_root(StorageRoot::LocalSlot(mir::LocalNodeId::new(0)));

        place.add_const_offset(-8);
        assert_eq!(place.const_offset, -8);

        place.add_const_offset(4);
        assert_eq!(place.const_offset, -4);
    }

    /// Equal ranges are detected.
    #[test]
    fn test_ranges_equal() {
        assert!(ByteRange::new(0, 4).equals(ByteRange::new(0, 4)));
        assert!(!ByteRange::new(0, 4).equals(ByteRange::new(0, 8)));
        assert!(!ByteRange::new(0, 4).equals(ByteRange::new(4, 4)));
        assert!(!ByteRange::new(0, 8).equals(ByteRange::new(4, 8)));
    }

    /// Adjacent ranges are disjoint.
    #[test]
    fn test_range_relation_adjacent() {
        assert_eq!(
            ByteRange::new(0, 4).relation(ByteRange::new(4, 4)),
            RangeRelation::Disjoint
        );
        assert_eq!(
            ByteRange::new(4, 4).relation(ByteRange::new(0, 4)),
            RangeRelation::Disjoint
        );
    }

    /// Partial overlaps are classified correctly.
    #[test]
    fn test_range_relation_partial_overlap() {
        assert_eq!(
            ByteRange::new(0, 8).relation(ByteRange::new(4, 8)),
            RangeRelation::Overlaps
        );
        assert_eq!(
            ByteRange::new(4, 8).relation(ByteRange::new(0, 8)),
            RangeRelation::Overlaps
        );
    }

    /// Zero sized ranges use the half open convention.
    #[test]
    fn test_range_relation_zero_size() {
        assert_eq!(
            ByteRange::new(0, 0).relation(ByteRange::new(0, 0)),
            RangeRelation::Disjoint
        );

        assert_eq!(
            ByteRange::new(0, 0).relation(ByteRange::new(0, 4)),
            RangeRelation::Disjoint
        );
        assert_eq!(
            ByteRange::new(0, 4).relation(ByteRange::new(0, 0)),
            RangeRelation::Disjoint
        );

        assert_eq!(
            ByteRange::new(2, 0).relation(ByteRange::new(0, 8)),
            RangeRelation::ContainedBy
        );

        // handle zero size after the end
        assert_eq!(
            ByteRange::new(10, 0).relation(ByteRange::new(0, 8)),
            RangeRelation::Disjoint
        );
    }
}
