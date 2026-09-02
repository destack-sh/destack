use std::sync::Arc;

use crate::{
    Analysis, Constant, ConstantTable, DefinitionTable, Function, FunctionCache, Global,
    MemoryLocation, MemoryRegion, Mutation, NodeTable, Place, PlaceOrigin, PlaceTable, Projection,
    Space, Storage, StorageRoot, TargetLayout, Tree, Value,
};

use super::MemoryRegionBuilder;

/// Alias analysis for one MIR function.
#[derive(Debug)]
pub struct AliasTable {
    /// Memory region indexed by SSA value id.
    regions: Vec<Option<MemoryRegion>>,
    /// Constants used to compare structural projections.
    constants: Arc<ConstantTable>,
    /// Whether each SSA value may alias another storage root.
    value_is_aliasable: Vec<bool>,
    /// Addressed storage indexed by SSA value.
    value_storage: Vec<Option<Storage>>,
    /// Static storage indexed by global identity.
    global_spaces: NodeTable<Global, Option<Space>>,
}

impl AliasTable {
    /// Build alias analysis for one function.
    pub fn build(
        function: &Function,
        definitions: &DefinitionTable,
        constants: Arc<ConstantTable>,
        places: &PlaceTable,
        target_layout: TargetLayout,
        tree: &Tree,
    ) -> Self {
        let regions = Self::build_regions(function, definitions, target_layout, tree);
        // index global storage by local node identity
        let globals = tree
            .iter_nodes::<Global>()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        let mut global_spaces = NodeTable::from_nodes(&globals, || None);
        for (id, global) in tree.iter_nodes::<Global>() {
            *global_spaces.get_mut(id) = Some(global.space);
        }

        // classify SSA roots that may alias other storage
        let value_is_aliasable = function
            .value_types()
            .iter()
            .map(|ty| {
                let Some(ty) = ty else {
                    return false;
                };
                let ty = tree.get(*ty);

                ty.is_aliasable_reference()
            })
            .collect();

        // resolve the storage addressed by every SSA value
        let value_storage = function
            .value_types()
            .iter()
            .enumerate()
            .map(|(index, ty)| {
                let value = Value::new(index as u32);

                match places.get(value).origin {
                    PlaceOrigin::Local(_) => Some(Storage::Frame),
                    PlaceOrigin::Global(global) => global_spaces.get(global).map(Storage::global),
                    PlaceOrigin::Value(_) => ty.and_then(|ty| tree.get(ty).reference_storage()),
                }
            })
            .collect();

        Self {
            regions,
            constants,
            value_is_aliasable,
            value_storage,
            global_spaces,
        }
    }

    /// Return whether two places may overlap.
    pub fn may_overlap(&self, left: &Place, right: &Place) -> bool {
        // compare structural paths rooted in the same storage
        if left.origin == right.origin {
            return !left
                .path
                .projections
                .iter()
                .zip(&right.path.projections)
                .any(|(left, right)| self.projections_are_disjoint(left, right));
        }

        // compare distinct storage roots through alias analysis
        self.origins_may_overlap(left.origin, right.origin)
    }

    /// Return the storage containing one place.
    pub fn storage(&self, place: &Place) -> Option<Storage> {
        self.origin_storage(place.origin)
    }

    /// Query whether two memory locations alias.
    pub fn alias(&self, left: &MemoryLocation, right: &MemoryLocation) -> AliasResult {
        if left.address == right.address {
            return left.alias_same_address(right);
        }

        // retain distinct projections from the same runtime address
        if left.address.value() == right.address.value() {
            return AliasResult::MayAlias;
        }

        let left_region = self.region(left.address.value());
        let right_region = self.region(right.address.value());

        match (left_region, right_region) {
            (MemoryRegion::Place(left_place), MemoryRegion::Place(right_place)) => {
                if left_place.root.is_disjoint_from(&right_place.root) {
                    return AliasResult::NoAlias;
                }

                if left_place
                    .root
                    .exclusive_parameters_are_disjoint(&right_place.root)
                {
                    return AliasResult::NoAlias;
                }

                if left_place.root == right_place.root {
                    return left_place.alias_with(left, right_place, right);
                }
            }
            _ => {
                let left_spaces = left_region.spaces();
                let right_spaces = right_region.spaces();

                if left_spaces.is_disjoint(right_spaces) {
                    return AliasResult::NoAlias;
                }
            }
        }

        AliasResult::MayAlias
    }

    /// Return true when two address values may alias.
    pub fn addresses_may_alias(&self, left: Value, right: Value) -> bool {
        let left = MemoryLocation::from_address(left);
        let right = MemoryLocation::from_address(right);

        self.alias(&left, &right).may_alias()
    }

    /// Return true when two address values definitely do not alias.
    pub fn addresses_no_alias(&self, left: Value, right: Value) -> bool {
        let left = MemoryLocation::from_address(left);
        let right = MemoryLocation::from_address(right);

        self.alias(&left, &right).is_no_alias()
    }

    /// Return whether an addressed location may touch one storage root.
    pub fn may_touch_root(&self, location: &MemoryLocation, storage: &StorageRoot) -> bool {
        self.region(location.address.value())
            .may_touch_root(storage)
    }

    /// Resolve one address value into a memory region.
    pub fn region(&self, address: Value) -> &MemoryRegion {
        let Some(region) = self
            .regions
            .get(address.id() as usize)
            .and_then(Option::as_ref)
        else {
            unreachable!("missing memory region for address value: {address:?}");
        };

        region
    }

    /// Resolve address origin in one memory region.
    pub fn resolve(&self, region: &MemoryRegion) -> MemoryRegion {
        match region.location() {
            Some(location) => self.region(location.address.value()).clone(),
            None => region.clone(),
        }
    }

    /// Return whether two distinct place origins may overlap.
    fn origins_may_overlap(&self, left: PlaceOrigin, right: PlaceOrigin) -> bool {
        match (left, right) {
            // compare two reference-carrying SSA roots
            (PlaceOrigin::Value(left), PlaceOrigin::Value(right)) => {
                if self.addresses_no_alias(left, right) {
                    return false;
                }

                self.value_may_overlap(left)
                    && self.value_may_overlap(right)
                    && self.value_storage(left) == self.value_storage(right)
            }
            // compare an opaque reference with concrete storage
            (PlaceOrigin::Value(value), concrete) | (concrete, PlaceOrigin::Value(value)) => {
                self.value_storage(value) == self.origin_storage(concrete)
            }
            // distinguish concrete local and global roots
            (PlaceOrigin::Local(_) | PlaceOrigin::Global(_), _) => false,
        }
    }

    /// Return whether one SSA root may overlap another storage root.
    fn value_may_overlap(&self, value: Value) -> bool {
        self.value_is_aliasable[value.id() as usize]
    }

    /// Return the storage addressed by one SSA value.
    fn value_storage(&self, value: Value) -> Option<Storage> {
        self.value_storage[value.id() as usize]
    }

    /// Return the storage represented by one concrete origin.
    fn origin_storage(&self, origin: PlaceOrigin) -> Option<Storage> {
        match origin {
            PlaceOrigin::Local(_) => Some(Storage::Frame),
            PlaceOrigin::Global(global) => self.global_spaces.get(global).map(Storage::global),
            PlaceOrigin::Value(value) => self.value_storage(value),
        }
    }

    /// Return whether two projections are proven disjoint.
    fn projections_are_disjoint(&self, left: &Projection, right: &Projection) -> bool {
        let Some(left) = self.projection_interval(left) else {
            return false;
        };
        let Some(right) = self.projection_interval(right) else {
            return false;
        };

        left.1 <= right.0 || right.1 <= left.0
    }

    /// Return the half-open index interval covered by one projection.
    fn projection_interval(&self, projection: &Projection) -> Option<(u128, u128)> {
        match projection {
            Projection::Field { index } | Projection::Element { index } => {
                let start = u128::from(*index);
                let end = start.checked_add(1)?;

                Some((start, end))
            }
            Projection::Index { index } => {
                let start = self.projection_constant(*index)?;
                let end = start.checked_add(1)?;

                Some((start, end))
            }
            Projection::Variant { .. } => None,
            Projection::Slice { start, length } => {
                let start = self.projection_constant(*start)?;
                let length = self.projection_constant(*length)?;
                let end = start.checked_add(length)?;

                Some((start, end))
            }
        }
    }

    /// Return the integer constant defined for one value.
    fn projection_constant(&self, value: Value) -> Option<u128> {
        match self.constants.constant(value)? {
            Constant::Int { value, .. } => u128::try_from(*value).ok(),
            Constant::UInt { value, .. } => Some(*value),
            _ => None,
        }
    }

    /// Build all memory regions for one function.
    fn build_regions(
        function: &Function,
        definitions: &DefinitionTable,
        target_layout: TargetLayout,
        tree: &Tree,
    ) -> Vec<Option<MemoryRegion>> {
        let mut builder = MemoryRegionBuilder::new(function, definitions, tree, target_layout);

        let mut regions = vec![None; function.value_capacity()];

        // resolve each defined SSA value once
        for (value, _) in definitions.definitions() {
            regions[value.id() as usize] = Some(builder.region(value));
        }

        regions
    }
}

impl Analysis for AliasTable {
    const INVALIDATED_BY: Mutation = Mutation::CONTROL
        .union(Mutation::VALUE)
        .union(Mutation::LAYOUT);
}

impl AliasTable {
    /// Compute aliases for one function.
    pub(crate) fn compute(function: &Function, tree: &Tree, analyses: &mut FunctionCache) -> Self {
        let definitions = analyses.definition(function, tree);
        let constants = analyses.constant(function, tree);
        let places = analyses.place(function, tree);

        Self::build(
            function,
            &definitions,
            constants,
            &places,
            analyses.target_layout(),
            tree,
        )
    }
}

/// Result of an alias query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AliasResult {
    /// The locations definitely refer to the same memory.
    MustAlias,
    /// The locations partially overlap.
    PartialAlias,
    /// The locations might refer to the same memory.
    MayAlias,
    /// The locations definitely do not overlap.
    NoAlias,
}

impl AliasResult {
    /// Return whether this proves non-aliasing.
    pub fn is_no_alias(self) -> bool {
        matches!(self, AliasResult::NoAlias)
    }

    /// Return whether the locations may alias.
    pub fn may_alias(self) -> bool {
        !matches!(self, AliasResult::NoAlias)
    }

    /// Return whether this proves identical memory.
    pub fn is_must_alias(self) -> bool {
        matches!(self, AliasResult::MustAlias)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::Instruction;
    use crate::analyses::tests::TestProgram;

    /// Independent local slots do not alias.
    #[test]
    fn test_alias_distinguishes_local_slots() {
        let program = TestProgram::new(
            r#"
function test(): int32 {
    local l0: int32
    local l1: int32

entry:
    v0: ref<int32, borrowed, mutable, frame> = local.address l0
    v1: ref<int32, borrowed, mutable, frame> = local.address l1
    v2: int32 = 0
    return v2
}
"#,
        );

        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let mut analyses = program.function_analyses();
        let alias = analyses.alias(function, &program.tree);
        let addresses = program.local_address_destinations_in_entry(function_id);

        // compare two distinct storage roots
        let left = MemoryLocation::from_address(addresses[0]);
        let right = MemoryLocation::from_address(addresses[1]);

        assert_eq!(alias.alias(&left, &right), AliasResult::NoAlias);
    }

    /// Constant propagation distinguishes projected array elements.
    #[test]
    fn test_alias_distinguishes_propagated_indices() {
        let program = TestProgram::new(
            r#"
function test(v0: ref<[int32; 4], borrowed, mutable>): void {
entry(v0: ref<[int32; 4], borrowed, mutable>):
    v1: uint64 = 0
    v2: uint64 = 1
    v3: uint64 = add v1, v2
    v4: ref<int32, borrowed, mutable> = element.address v0, v1
    v5: ref<int32, borrowed, mutable> = element.address v0, v3
    return
}
"#,
        );

        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let mut analyses = program.function_analyses();
        let alias = analyses.alias(function, &program.tree);
        let places = analyses.place(function, &program.tree);
        let addresses = program
            .entry_instructions(function_id)
            .into_iter()
            .filter_map(|instruction| match program.tree.get(instruction) {
                Instruction::ElementAddr { destination, .. } => Some(*destination),
                _ => None,
            })
            .collect::<Vec<_>>();

        // compare projections using direct and propagated constants
        assert!(!alias.may_overlap(places.get(addresses[0]), places.get(addresses[1])));
    }
}
