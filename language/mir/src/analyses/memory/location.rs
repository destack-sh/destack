use tspp_core::FxIndexMap;

use crate as mir;

use crate::{AliasResult, DefinitionTable, DominatorTable, KnownBits, ValueDefinition};

/// Maximum definition depth inspected while decomposing one address.
const MAX_ADDRESS_DEPTH: usize = 6;

/// Extent of one memory access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemorySize {
    /// An exact number of bytes.
    Bytes(u64),
    /// The canonical layout size of a stored value.
    Type(mir::TypeId),
    /// A byte count determined at runtime.
    Dynamic,
}

impl MemorySize {
    /// Resolve a typed extent using the canonical layout table.
    pub fn byte_len(self, layouts: &mir::LayoutTable) -> Result<Option<u64>, mir::LayoutError> {
        match self {
            Self::Bytes(bytes) => Ok(Some(bytes)),
            Self::Type(ty) => {
                let layout = layouts
                    .type_layout(ty)
                    .ok_or(mir::LayoutError::Missing { ty })?;

                Ok(Some(u64::from(layout.size)))
            }
            Self::Dynamic => Ok(None),
        }
    }
}

impl From<Option<u64>> for MemorySize {
    /// Preserve a known byte count or its runtime extent.
    fn from(size: Option<u64>) -> Self {
        size.map_or(Self::Dynamic, Self::Bytes)
    }
}

/// One MIR address used by memory analysis.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MemoryAddress {
    /// Storage selected by an explicit place.
    Place(mir::Place),
    /// A field selected from one dynamic value.
    Dynamic {
        /// The dynamic value carrying the payload and dispatch table.
        value: mir::Value,
        /// The dispatch slot selecting the field offset.
        slot: mir::DispatchSlot,
    },
}

impl MemoryAddress {
    /// Return whether the address depends only on immutable SSA values and fixed storage.
    pub fn is_stable(&self) -> bool {
        match self {
            Self::Dynamic { .. } => true,
            Self::Place(place) => place.is_stable(),
        }
    }

    /// Return the SSA reference at the root, when the address has one.
    pub fn value(&self) -> Option<mir::Value> {
        match self {
            Self::Place(place) => match place.origin {
                mir::PlaceOrigin::Value(value)
                    if place.path.first() == Some(&mir::Projection::Deref) =>
                {
                    Some(value)
                }
                _ => None,
            },
            Self::Dynamic { value, .. } => Some(*value),
        }
    }
}

impl From<mir::Value> for MemoryAddress {
    /// Select the referent of an SSA address.
    fn from(value: mir::Value) -> Self {
        Self::Place(mir::Place::value(value).with_projection(mir::Projection::Deref))
    }
}

impl From<mir::Place> for MemoryAddress {
    /// Preserve an explicit place operand.
    fn from(place: mir::Place) -> Self {
        Self::Place(place)
    }
}

/// One memory location reached through an address.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryLocation {
    /// The address being dereferenced.
    pub address: MemoryAddress,
    /// The byte count, stored type, or runtime extent of the access.
    pub size: MemorySize,
    /// The value type being accessed, when known.
    pub value_type: Option<mir::TypeId>,
    /// Reference kind for the address, when known.
    pub reference_kind: Option<mir::Reference>,
    /// Storage for the address, when known.
    pub reference_storage: Option<mir::Storage>,
}

impl MemoryLocation {
    /// Create a location from an address with unknown size.
    pub fn from_address(address: impl Into<MemoryAddress>) -> Self {
        Self {
            address: address.into(),
            size: MemorySize::Dynamic,
            value_type: None,
            reference_kind: None,
            reference_storage: None,
        }
    }

    /// Create a location with known size.
    pub fn with_size(address: impl Into<MemoryAddress>, size: u64) -> Self {
        Self {
            address: address.into(),
            size: MemorySize::Bytes(size),
            value_type: None,
            reference_kind: None,
            reference_storage: None,
        }
    }

    /// Create a fully specified location.
    pub fn new(
        address: impl Into<MemoryAddress>,
        size: MemorySize,
        value_type: Option<mir::TypeId>,
        reference_kind: Option<mir::Reference>,
        reference_storage: Option<mir::Storage>,
    ) -> Self {
        Self {
            address: address.into(),
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

    /// Return whether both accesses store the same type and number of bytes.
    pub fn has_compatible_value(
        &self,
        other: &MemoryLocation,
        layouts: &mir::LayoutTable,
    ) -> Result<bool, mir::LayoutError> {
        let left = self.size.byte_len(layouts)?;
        let right = other.size.byte_len(layouts)?;

        // require equal concrete extents and stored types
        let is_same_size = matches!((left, right), (Some(left), Some(right)) if left == right);
        let types = (self.value_type, other.value_type);
        let is_same_type = matches!(types, (Some(left), Some(right)) if left == right);

        Ok(is_same_size && is_same_type)
    }
}

/// Memory region touched by one address value or direct access.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MemoryRegion {
    /// Memory access through an address value.
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

    /// Return whether this region is owned by the current activation frame.
    pub fn is_frame_storage(&self) -> bool {
        matches!(self, Self::Local(_))
            || matches!(self, Self::Place(place) if matches!(place.root, StorageRoot::LocalSlot(_)))
    }

    /// Create an address access covering one typed value.
    pub fn from_address(
        address: impl Into<MemoryAddress>,
        value_type: Option<mir::TypeId>,
        reference_kind: Option<mir::Reference>,
        reference_storage: Option<mir::Storage>,
    ) -> Self {
        let size = value_type.map_or(MemorySize::Dynamic, MemorySize::Type);

        Self::Address {
            location: MemoryLocation::new(
                address,
                size,
                value_type,
                reference_kind,
                reference_storage,
            ),
            spaces: mir::StorageSet::ANY,
        }
    }

    /// Create an address access with an explicit or runtime byte count.
    pub fn from_address_with_size(
        address: impl Into<MemoryAddress>,
        value_type: Option<mir::TypeId>,
        reference_kind: Option<mir::Reference>,
        reference_storage: Option<mir::Storage>,
        size: Option<u64>,
    ) -> Self {
        Self::Address {
            location: MemoryLocation::new(
                address,
                size.into(),
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

    /// Return whether two accessed regions can overlap.
    pub fn may_alias(
        &self,
        other: &Self,
        alias: &mir::AliasTable,
    ) -> Result<bool, mir::LayoutError> {
        // reject disjoint memory spaces before inspecting the addressed storage
        if self.spaces().is_disjoint(other.spaces()) {
            return Ok(false);
        }

        // compare addresses precisely and opaque accesses by their possible storage
        Ok(match (self, other) {
            (Self::Any { .. }, _) | (_, Self::Any { .. }) => true,
            (
                Self::Address { location: left, .. },
                Self::Address {
                    location: right, ..
                },
            ) => alias.alias(left, right)?.may_alias(),
            (Self::Address { location, .. }, Self::Local(local))
            | (Self::Local(local), Self::Address { location, .. }) => {
                alias.may_touch_root(location, &StorageRoot::LocalSlot(*local))?
            }
            (Self::Address { location, .. }, Self::Place(place))
            | (Self::Place(place), Self::Address { location, .. }) => {
                alias.may_touch_root(location, &place.root)?
            }
            (Self::Local(local), region) | (region, Self::Local(local)) => {
                region.may_touch_root(&StorageRoot::LocalSlot(*local))
            }
            (Self::Place(left), Self::Place(right)) => !left.root.is_disjoint_from(&right.root),
        })
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

/// Base address for one memory place.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StorageRoot {
    /// Local slot address.
    LocalSlot(mir::LocalNodeId<mir::Local>),
    /// Global storage address.
    Global {
        /// The global declaration.
        global: mir::LocalNodeId<mir::Global>,
        /// The possible storage spaces.
        spaces: mir::StorageSet,
    },
    /// Storage created by one allocation operation.
    Allocation {
        /// The allocation operation.
        point: mir::Point,
        /// The block that executes the allocation.
        block: mir::BlockId,
        /// The possible storage spaces.
        spaces: mir::StorageSet,
    },
    /// An address whose allocation is unknown.
    Address {
        /// The place selecting the base address.
        place: mir::Place,
        /// The memory spaces permitted by its type.
        spaces: mir::StorageSet,
    },
    /// Function parameter.
    Parameter {
        /// Parameter index.
        index: u32,
        /// The memory spaces permitted by the parameter type.
        spaces: mir::StorageSet,
    },
}

impl StorageRoot {
    /// Return whether two identified storage roots are disjoint.
    pub fn is_disjoint_from(&self, other: &StorageRoot) -> bool {
        // exclude storage spaces that cannot share addresses
        if self.spaces().is_disjoint(other.spaces()) {
            return true;
        }

        // compare identified storage roots
        match (self, other) {
            (StorageRoot::Address { .. }, _) | (_, StorageRoot::Address { .. }) => false,
            (StorageRoot::LocalSlot(left), StorageRoot::LocalSlot(right)) => left != right,
            (
                StorageRoot::Global { global: left, .. },
                StorageRoot::Global { global: right, .. },
            ) => left != right,
            (
                StorageRoot::Allocation { point: left, .. },
                StorageRoot::Allocation { point: right, .. },
            ) => left != right,
            (StorageRoot::Parameter { .. }, StorageRoot::Allocation { .. })
            | (StorageRoot::Allocation { .. }, StorageRoot::Parameter { .. }) => true,
            (StorageRoot::Parameter { .. }, _) | (_, StorageRoot::Parameter { .. }) => false,
            _ => true,
        }
    }

    /// Return the memory spaces covered by this storage root.
    pub fn spaces(&self) -> mir::StorageSet {
        match self {
            StorageRoot::LocalSlot(_) => mir::StorageSet::FRAME,
            StorageRoot::Global { spaces, .. }
            | StorageRoot::Allocation { spaces, .. }
            | StorageRoot::Parameter { spaces, .. }
            | StorageRoot::Address { spaces, .. } => *spaces,
        }
    }
}

/// Indexed byte offset component in address arithmetic.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IndexedOffset {
    /// The index value.
    pub index: mir::Value,
    /// Scale factor (element size in bytes).
    pub scale: i128,
}

/// Precise memory place representation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryPlace {
    /// The identified storage root.
    pub root: StorageRoot,
    /// Constant byte offset from storage root.
    pub const_offset: i128,
    /// Indexed offsets with their scales.
    pub indexed_offsets: Vec<IndexedOffset>,
}

impl MemoryPlace {
    /// Create a memory place from one storage root.
    pub fn from_root(root: StorageRoot) -> Self {
        Self {
            root,
            const_offset: 0,
            indexed_offsets: Vec::new(),
        }
    }

    /// Return whether this place has only constant offsets.
    pub fn is_constant_offset(&self) -> bool {
        self.indexed_offsets.is_empty()
    }

    /// Add a constant offset.
    pub fn add_const_offset(&mut self, offset: i128) {
        self.const_offset = self.const_offset.wrapping_add(offset);
    }

    /// Add an indexed offset.
    pub fn add_indexed_offset(&mut self, index: mir::Value, scale: i128) {
        // combine equal indices and keep a stable order for address comparisons
        if let Some(offset) = self
            .indexed_offsets
            .iter_mut()
            .find(|offset| offset.index == index)
        {
            offset.scale = offset.scale.wrapping_add(scale);
        } else {
            self.indexed_offsets.push(IndexedOffset { index, scale });
        }
        self.indexed_offsets.retain(|offset| offset.scale != 0);
        self.indexed_offsets
            .sort_unstable_by_key(|offset| offset.index.id());
    }

    /// Subtract the symbolic offsets of another place and cancel equal terms.
    pub(super) fn difference<'a>(
        &'a self,
        other: &'a Self,
    ) -> impl Iterator<Item = IndexedOffset> + 'a {
        let left = self.indexed_offsets.iter().map(|left| {
            let right = other
                .indexed_offsets
                .iter()
                .find(|right| right.index == left.index);
            let scale = right.map_or(left.scale, |right| left.scale.wrapping_sub(right.scale));

            IndexedOffset {
                index: left.index,
                scale,
            }
        });
        let right = other
            .indexed_offsets
            .iter()
            .filter(|right| {
                !self
                    .indexed_offsets
                    .iter()
                    .any(|left| left.index == right.index)
            })
            .map(|right| IndexedOffset {
                index: right.index,
                scale: right.scale.wrapping_neg(),
            });

        left.chain(right).filter(|offset| offset.scale != 0)
    }

    /// Return aliasing with another place under known access locations.
    pub fn alias_with(
        &self,
        size: Option<u64>,
        other: &MemoryPlace,
        other_size: Option<u64>,
        pointer_bits: u16,
    ) -> AliasResult {
        // exclude accesses that touch no bytes
        if size == Some(0) || other_size == Some(0) {
            return AliasResult::NoAlias;
        }

        // cancel matching symbolic terms in the target pointer width
        let alignment = self
            .difference(other)
            .fold(u32::from(pointer_bits), |bits, offset| {
                bits.min(offset.scale.trailing_zeros())
            });

        // compare byte offsets modulo the common power of two in every remaining term
        let modulus = 1u128 << alignment;
        let offset = self.const_offset.wrapping_sub(other.const_offset) as u128 & (modulus - 1);
        let is_exact = alignment == u32::from(pointer_bits);
        if is_exact && offset == 0 {
            return AliasResult::MustAlias;
        }

        // exclude overlap in both directions around the address modulus
        match (size, other_size) {
            (Some(size), Some(other_size)) => {
                if offset >= u128::from(other_size) && modulus - offset >= u128::from(size) {
                    AliasResult::NoAlias
                } else if is_exact {
                    AliasResult::PartialAlias
                } else {
                    AliasResult::MayAlias
                }
            }
            _ => AliasResult::MayAlias,
        }
    }
}

/// Resolve memory regions through SSA definitions.
#[derive(Debug)]
pub(super) struct MemoryRegionBuilder<'a> {
    /// Cached region results.
    cache: FxIndexMap<mir::Value, MemoryRegion>,
    /// Value definitions for address origin.
    definitions: &'a DefinitionTable,
    /// The MIR tree.
    pub(super) tree: &'a mir::Tree,
    /// The MIR function.
    function: mir::FunctionId,
    /// Dominance used to distinguish values from different loop iterations.
    dominators: &'a DominatorTable,
    /// Integer constants used in address arithmetic.
    constants: &'a mir::ConstantTable,
    /// Canonical layouts for the analysed representation.
    layouts: &'a mir::LayoutTable,
    /// The target integer and pointer widths.
    target: mir::TargetLayout,
}

impl<'a> MemoryRegionBuilder<'a> {
    /// Create a new region builder.
    pub(super) fn new(
        function: mir::FunctionId,
        definitions: &'a DefinitionTable,
        dominators: &'a DominatorTable,
        tree: &'a mir::Tree,
        layouts: &'a mir::LayoutTable,
        constants: &'a mir::ConstantTable,
        target: mir::TargetLayout,
    ) -> Self {
        Self {
            cache: FxIndexMap::default(),
            definitions,
            tree,
            function,
            dominators,
            layouts,
            constants,
            target,
        }
    }

    /// Resolve an explicit place into storage and byte offsets.
    pub(super) fn place(
        &mut self,
        operand: &mir::Place,
        depth: usize,
    ) -> Result<MemoryRegion, mir::LayoutError> {
        let steps = self
            .layouts
            .address_steps(operand, self.function, self.tree)?;
        let mut prefix = mir::Place::new(operand.origin);
        let root = match operand.origin {
            mir::PlaceOrigin::Local(local) => StorageRoot::LocalSlot(local),
            mir::PlaceOrigin::Global(global) => StorageRoot::Global {
                global,
                spaces: mir::Storage::global(self.tree.get(global).space).storage_set(),
            },
            mir::PlaceOrigin::Value(_) => StorageRoot::Address {
                place: prefix.clone(),
                spaces: mir::StorageSet::FRAME,
            },
        };
        let mut region = MemoryRegion::Place(MemoryPlace::from_root(root));

        // apply the address step of each projection
        for (projection, step) in operand.path.projections.iter().zip(steps) {
            match step {
                // resolve a root address value, else address the referent of the stored reference
                mir::AddressStep::Follow(descriptor) => {
                    if prefix.path.is_root()
                        && let mir::PlaceOrigin::Value(value) = prefix.origin
                    {
                        region = self.region(value, depth + 1)?;
                    } else {
                        let spaces = self
                            .tree
                            .type_definition(descriptor.ty)
                            .reference_storage_set()
                            .unwrap_or(mir::StorageSet::ANY);
                        let place = prefix.clone().with_projection(mir::Projection::Deref);
                        region =
                            MemoryRegion::Place(MemoryPlace::from_root(StorageRoot::Address {
                                place,
                                spaces,
                            }));
                    }
                }
                // advance by one field, payload, or fixed element offset
                mir::AddressStep::Offset(offset) => {
                    if let MemoryRegion::Place(place) = &mut region {
                        place.add_const_offset(i128::from(offset));
                    }
                }
                // scale one runtime index
                mir::AddressStep::Index { index, stride, .. } => {
                    self.add_index(&mut region, index, u64::from(stride));
                }
            }
            prefix.push(projection.clone());
        }

        Ok(region)
    }

    /// Resolve an address value to a memory region.
    pub(super) fn region(
        &mut self,
        address: mir::Value,
        depth: usize,
    ) -> Result<MemoryRegion, mir::LayoutError> {
        // check cache
        if let Some(cached) = self.cache.get(&address) {
            return Ok(cached.clone());
        }

        // bound cyclic block arguments and long address expressions
        if depth >= MAX_ADDRESS_DEPTH {
            return Ok(self.address_region(address));
        }

        // resolve and cache the address definition
        let result = self.resolve_definition(address, depth)?;
        self.cache.insert(address, result.clone());
        Ok(result)
    }

    /// Resolve an address value to a memory region.
    fn resolve_definition(
        &mut self,
        address: mir::Value,
        depth: usize,
    ) -> Result<MemoryRegion, mir::LayoutError> {
        // resolve parameters and incoming block arguments before instruction definitions
        let instruction_id = match self.definitions.definition(address) {
            Some(ValueDefinition::FunctionParameter(index)) => {
                let is_unchanged = self
                    .definitions
                    .inputs(address)
                    .iter()
                    .all(|input| input.argument == Some(address));
                return Ok(if is_unchanged {
                    self.parameter_region(index, &self.tree.get(self.function).parameters[index])
                } else {
                    self.address_region(address)
                });
            }
            Some(ValueDefinition::BlockParameter { .. }) => {
                let mut merged = None;
                for input in self.definitions.inputs(address) {
                    let region = if let Some(argument) = input.argument {
                        if argument == address {
                            continue;
                        }
                        self.region(argument, depth + 1)?
                    } else if input.edge.successor == mir::Successor::NewSuccess {
                        self.allocation_region(mir::Point::Terminator(input.edge.source), address)
                    } else {
                        self.address_region(address)
                    };
                    // require allocation identities and symbolic indices to precede the merge
                    if let MemoryRegion::Place(place) = &region {
                        let base = match place.root {
                            StorageRoot::Address { ref place, .. } => {
                                if place
                                    .uses()
                                    .iter()
                                    .any(|value| !self.is_available(*value, input.edge.target))
                                {
                                    return Ok(self.address_region(address));
                                }
                                None
                            }
                            StorageRoot::Allocation {
                                point: mir::Point::Instruction(instruction),
                                ..
                            } => self.tree.get(instruction).destination(),
                            _ => None,
                        };
                        if base.is_some_and(|value| !self.is_available(value, input.edge.target))
                            || place
                                .indexed_offsets
                                .iter()
                                .any(|offset| !self.is_available(offset.index, input.edge.target))
                        {
                            return Ok(self.address_region(address));
                        }
                    }
                    if merged.as_ref().is_some_and(|previous| previous != &region) {
                        return Ok(self.address_region(address));
                    }
                    merged = Some(region);
                }

                return Ok(merged.unwrap_or_else(|| self.address_region(address)));
            }
            Some(ValueDefinition::Instruction { instruction, .. }) => instruction,
            None => unreachable!("address value has no MIR definition: {address:?}"),
        };

        // read the defining instruction
        let instruction = &self.tree.get(instruction_id).clone();

        // decompose allocations, projections, and address conversions
        let region = match instruction {
            mir::Instruction::NewZeroed { destination, .. }
            | mir::Instruction::NewUninit { destination, .. }
                if *destination == address =>
            {
                self.allocation_region(mir::Point::Instruction(instruction_id), address)
            }
            mir::Instruction::NewSliceZeroed { destination, .. }
            | mir::Instruction::NewSliceUninit { destination, .. }
                if *destination == address =>
            {
                self.allocation_region(mir::Point::Instruction(instruction_id), address)
            }
            mir::Instruction::NewComplete {
                destination, value, ..
            } if *destination == address => {
                let value = *value;

                self.region(value, depth + 1)?
            }

            mir::Instruction::Address { place, .. } => self.place(place, depth + 1)?,

            // preserve addresses through pointer bitcasts
            mir::Instruction::Cast {
                destination,
                argument,
                operator: mir::CastOperator::Bitcast,
                ..
            } if *destination == address
                && {
                    let ty = self.tree.get(self.function).expect_value_type(*argument);
                    let ty = mir::Substitution::resolve(ty, self.tree);
                    self.tree.get(ty).pointee_type()
                }
                .is_some()
                && {
                    let ty = self.tree.get(self.function).expect_value_type(address);
                    let ty = mir::Substitution::resolve(ty, self.tree);
                    self.tree.get(ty).pointee_type()
                }
                .is_some() =>
            {
                let argument = *argument;

                self.region(argument, depth + 1)?
            }

            // preserve an address selected from equivalent definitions
            mir::Instruction::Select {
                condition,
                then_value,
                else_value,
                ..
            } => match self.constants.constant(*condition) {
                Some(mir::Constant::Boolean { value }) => {
                    self.region(if *value { *then_value } else { *else_value }, depth + 1)?
                }
                _ => {
                    let left = self.region(*then_value, depth + 1)?;
                    let right = self.region(*else_value, depth + 1)?;

                    // preserve a common address across both selection arms
                    if left == right {
                        left
                    } else {
                        self.address_region(address)
                    }
                }
            },

            // preserve pointer representations through explicit memory intrinsics
            mir::Instruction::Intrinsic {
                intrinsic: mir::Intrinsic::Transmute | mir::Intrinsic::SpaceCast,
                arguments,
                ..
            } if {
                let ty = self.tree.get(self.function).expect_value_type(address);
                let ty = mir::Substitution::resolve(ty, self.tree);
                self.tree.get(ty).pointee_type()
            }
            .is_some() =>
            {
                let [argument] = *self.tree.get_values(*arguments) else {
                    unreachable!("pointer reinterpretation requires one MIR argument");
                };
                if {
                    let ty = self.tree.get(self.function).expect_value_type(argument);
                    let ty = mir::Substitution::resolve(ty, self.tree);
                    self.tree.get(ty).pointee_type()
                }
                .is_some()
                {
                    self.region(argument, depth + 1)?
                } else {
                    self.address_region(address)
                }
            }

            // anything else is imprecise
            _ => self.address_region(address),
        };

        Ok(region)
    }

    /// Return whether a value precedes the entry of one block.
    fn is_available(&self, value: mir::Value, block: mir::BlockId) -> bool {
        match self.definitions.definition(value) {
            Some(ValueDefinition::FunctionParameter(_)) => self
                .definitions
                .inputs(value)
                .iter()
                .all(|input| input.argument == Some(value)),
            Some(ValueDefinition::Instruction {
                block: definition, ..
            })
            | Some(ValueDefinition::BlockParameter {
                block: definition, ..
            }) => definition != block && self.dominators.dominates(definition, block),
            None => unreachable!("address component has no MIR definition: {value:?}"),
        }
    }

    /// Return a region for a parameter address.
    fn parameter_region(&self, index: usize, parameter: &mir::FunctionParameter) -> MemoryRegion {
        let ty = self.tree.get(parameter.ty);

        MemoryRegion::Place(MemoryPlace::from_root(StorageRoot::Parameter {
            index: index as u32,
            spaces: ty.reference_storage_set().unwrap_or(mir::StorageSet::ANY),
        }))
    }

    /// Return the storage created by one allocation.
    fn allocation_region(&mut self, point: mir::Point, address: mir::Value) -> MemoryRegion {
        let space = self.allocation_space(point);

        // locate the block that creates the allocation
        let block = match point {
            mir::Point::Instruction(_) => self
                .definitions
                .block(address)
                .unwrap_or_else(|| unreachable!("allocation has no defining MIR block")),
            mir::Point::Terminator(block) => block,
        };

        MemoryRegion::Place(MemoryPlace::from_root(StorageRoot::Allocation {
            point,
            block,
            spaces: space.space_set(),
        }))
    }

    /// Return the heap one allocation point names.
    fn allocation_space(&self, point: mir::Point) -> mir::Space {
        let space = match point {
            mir::Point::Instruction(instruction) => self.tree.get(instruction).allocation_space(),
            mir::Point::Terminator(block) => self
                .tree
                .get(self.tree.get(block).terminator)
                .allocation_space(),
        };

        space.unwrap_or_else(|| unreachable!("allocation point outside an allocation"))
    }

    /// Retain an unknown allocation's base address and permitted memory spaces.
    fn address_region(&mut self, address: mir::Value) -> MemoryRegion {
        let ty = self.value_type(address);
        let ty = self.tree.get(ty);
        let spaces = ty.reference_storage_set().unwrap_or(mir::StorageSet::ANY);

        MemoryRegion::Place(MemoryPlace::from_root(StorageRoot::Address {
            place: mir::Place::value(address).with_projection(mir::Projection::Deref),
            spaces,
        }))
    }

    /// Return the value type for an SSA value.
    fn value_type(&mut self, value: mir::Value) -> mir::TypeId {
        let mut ty = mir::Substitution::resolve(
            self.tree.get(self.function).expect_value_type(value),
            self.tree,
        );
        while let mir::Type::Uninit { value } | mir::Type::ManuallyDrop { value } =
            self.tree.get(ty)
        {
            ty = mir::Substitution::resolve(*value, self.tree);
        }

        ty
    }

    /// Add an element index to a resolved address.
    fn add_index(&self, region: &mut MemoryRegion, index: mir::Value, scale: u64) {
        let MemoryRegion::Place(place) = region else {
            return;
        };
        if scale == 0 {
            return;
        }

        // expand integer arithmetic only while its mathematical value remains representable
        let mut result = place.clone();
        if self
            .expand_index(index, i128::from(scale), 0, &mut result)
            .is_some()
        {
            *place = result;
        } else {
            place.add_indexed_offset(index, i128::from(scale));
        }
    }

    /// Expand affine index arithmetic without distributing through integer overflow.
    fn expand_index(
        &self,
        index: mir::Value,
        scale: i128,
        depth: usize,
        place: &mut MemoryPlace,
    ) -> Option<()> {
        // fold exact constants into the byte displacement
        if let Some(value) = self.integer(index) {
            place.const_offset = place.const_offset.checked_add(value.checked_mul(scale)?)?;

            return Some(());
        }

        // preserve an opaque term when the expression exceeds the inspection budget
        let instruction = self
            .definitions
            .instruction(index)
            .map(|id| self.tree.get(id));
        if depth < MAX_ADDRESS_DEPTH {
            let arithmetic = match instruction {
                Some(mir::Instruction::Binary {
                    operator,
                    left,
                    right,
                    ..
                }) if self.does_not_wrap(*operator, *left, *right) => {
                    Some((*operator, *left, *right))
                }
                Some(mir::Instruction::Intrinsic {
                    intrinsic:
                        intrinsic @ (mir::Intrinsic::AddUnchecked
                        | mir::Intrinsic::SubUnchecked
                        | mir::Intrinsic::MulUnchecked),
                    arguments,
                    ..
                }) => {
                    let [left, right] = self.tree.get_values(*arguments) else {
                        unreachable!("unchecked arithmetic requires two operands");
                    };
                    let operator = match intrinsic {
                        mir::Intrinsic::AddUnchecked => mir::BinaryOperator::Add,
                        mir::Intrinsic::SubUnchecked => mir::BinaryOperator::Subtract,
                        _ => mir::BinaryOperator::Multiply,
                    };

                    Some((operator, *left, *right))
                }
                _ => None,
            };

            // distribute addition, subtraction, and multiplication by a constant
            if let Some((operator, left, right)) = arithmetic {
                match operator {
                    mir::BinaryOperator::Add
                    | mir::BinaryOperator::Subtract
                    | mir::BinaryOperator::Or => {
                        let right_scale = if operator == mir::BinaryOperator::Subtract {
                            scale.wrapping_neg()
                        } else {
                            scale
                        };
                        self.expand_index(left, scale, depth + 1, place)?;

                        return self.expand_index(right, right_scale, depth + 1, place);
                    }
                    mir::BinaryOperator::ShiftLeft => {
                        if let Some(shift) = self
                            .integer(right)
                            .and_then(|shift| u32::try_from(shift).ok())
                            && shift < 127
                        {
                            let factor = 1i128 << shift;

                            return self.expand_index(
                                left,
                                scale.checked_mul(factor)?,
                                depth + 1,
                                place,
                            );
                        }
                    }
                    mir::BinaryOperator::Multiply => {
                        let constant = self
                            .integer(right)
                            .map(|value| (left, value))
                            .or_else(|| self.integer(left).map(|value| (right, value)));
                        if let Some((value, factor)) = constant {
                            return self.expand_index(
                                value,
                                scale.checked_mul(factor)?,
                                depth + 1,
                                place,
                            );
                        }
                    }
                    _ => {}
                }
            }

            // preserve the numeric value through lossless integer widening
            if let Some(mir::Instruction::Cast {
                operator: mir::CastOperator::IntToInt,
                argument,
                to_type,
                ..
            }) = instruction
            {
                let source = self
                    .tree
                    .get(self.tree.get(self.function).expect_value_type(*argument));
                let target = self.tree.get(*to_type);
                let pointer_bits = self.target.pointer_bits();
                if let (Some((source_width, source_signed)), Some((target_width, target_signed))) =
                    (source.integer(pointer_bits), target.integer(pointer_bits))
                {
                    let preserves_value = (target_width >= source_width
                        && source_signed == target_signed)
                        || (target_width > source_width && !source_signed);
                    if preserves_value {
                        return self.expand_index(*argument, scale, depth + 1, place);
                    }
                }
            }
        }

        // combine repeated terms after checking the coefficient's representation
        if let Some(offset) = place
            .indexed_offsets
            .iter()
            .find(|offset| offset.index == index)
        {
            offset.scale.checked_add(scale)?;
        }
        place.add_indexed_offset(index, scale);

        Some(())
    }

    /// Return one integer constant as a signed mathematical value.
    fn integer(&self, value: mir::Value) -> Option<i128> {
        match self.constants.constant(value) {
            Some(mir::Constant::Int { value, .. }) => Some(*value),
            Some(mir::Constant::UInt { value, .. }) => i128::try_from(*value).ok(),
            _ => None,
        }
    }

    /// Check integer operand bounds before distributing arithmetic into byte offsets.
    fn does_not_wrap(
        &self,
        operator: mir::BinaryOperator,
        left: mir::Value,
        right: mir::Value,
    ) -> bool {
        let ty = self
            .tree
            .get(self.tree.get(self.function).expect_value_type(left));
        let Some((width, is_signed)) = ty.integer(self.target.pointer_bits()) else {
            return false;
        };
        let mask = u128::MAX >> (128 - width);
        let left = KnownBits::analyse(
            left,
            0,
            self.tree.get(self.function),
            self.definitions,
            self.target,
            self.tree,
        );
        let right = KnownBits::analyse(
            right,
            0,
            self.tree.get(self.function),
            self.definitions,
            self.target,
            self.tree,
        );

        // treat disjoint bit fields as an addition without carries
        if operator == mir::BinaryOperator::Or {
            return left.zero | right.zero == mask;
        }

        // treat a bounded left shift as multiplication by its power of two
        if operator == mir::BinaryOperator::ShiftLeft {
            if right.zero | right.one != mask || right.one >= u128::from(width) || right.one >= 127
            {
                return false;
            }
            let factor = 1u128 << right.one;
            if is_signed {
                let (minimum, maximum) = left.signed_range(width);
                let limit = (mask >> 1) as i128;

                return [minimum, maximum].into_iter().all(|value| {
                    value
                        .checked_mul(factor as i128)
                        .is_some_and(|value| (-limit - 1..=limit).contains(&value))
                });
            }

            return (!left.zero & mask)
                .checked_mul(factor)
                .is_some_and(|value| value <= mask);
        }

        // bound signed arithmetic in its declared width
        if is_signed {
            let (left_min, left_max) = left.signed_range(width);
            let (right_min, right_max) = right.signed_range(width);
            let limit = (mask >> 1) as i128;
            let values = match operator {
                mir::BinaryOperator::Add => [
                    left_min.checked_add(right_min),
                    left_max.checked_add(right_max),
                    Some(0),
                    Some(0),
                ],
                mir::BinaryOperator::Subtract => [
                    left_min.checked_sub(right_max),
                    left_max.checked_sub(right_min),
                    Some(0),
                    Some(0),
                ],
                mir::BinaryOperator::Multiply => [
                    left_min.checked_mul(right_min),
                    left_min.checked_mul(right_max),
                    left_max.checked_mul(right_min),
                    left_max.checked_mul(right_max),
                ],
                _ => return false,
            };

            values
                .into_iter()
                .all(|value| value.is_some_and(|value| (-limit - 1..=limit).contains(&value)))
        }
        // bound unsigned arithmetic without converting its full range to signed integers
        else {
            let maximum = match operator {
                mir::BinaryOperator::Add => (!left.zero & mask).checked_add(!right.zero & mask),
                mir::BinaryOperator::Subtract if left.one >= !right.zero & mask => {
                    Some(!left.zero & mask)
                }
                mir::BinaryOperator::Multiply => {
                    (!left.zero & mask).checked_mul(!right.zero & mask)
                }
                _ => return false,
            };

            maximum.is_some_and(|value| value <= mask)
        }
    }
}
