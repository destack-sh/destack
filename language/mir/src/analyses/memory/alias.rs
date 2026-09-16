use std::sync::Arc;

use destack_core::FxIndexMap;
use smallvec::SmallVec;

use crate::{
    Analysis, BlockId, ConstantTable, DefinitionTable, DominatorTable, FunctionId, Instruction,
    KnownBits, LayoutError, LayoutTable, MemoryAddress, MemoryLocation, MemoryPlace, MemoryRegion,
    MemorySize, Mutation, Place, PlaceOrigin, Projection, StorageRoot, TargetLayout, Tree, Value,
    ValueDefinition,
};

use super::MemoryRegionBuilder;

/// Maximum recursive alias comparisons in one query.
const QUERY_LIMIT: usize = 100;

/// Alias analysis for one MIR function.
#[derive(Debug)]
pub struct AliasTable {
    /// Canonical layouts shared by physical memory queries.
    pub(super) layouts: Arc<LayoutTable>,
    /// Value definitions used when a memory query crosses a control-flow merge.
    definitions: Arc<DefinitionTable>,
    /// Dominance of the values used by an address.
    dominators: Arc<DominatorTable>,
    /// Memory region indexed by SSA value id.
    regions: Vec<Option<MemoryRegion>>,
    /// Physical regions selected by explicit memory operands.
    operands: FxIndexMap<Place, MemoryRegion>,
    /// Conditional address definitions whose arms can be compared together.
    selections: FxIndexMap<Value, Selection>,
    /// The target integer and pointer widths.
    target: TargetLayout,
    /// Integer bounds for symbolic address indices.
    bounds: FxIndexMap<Value, (i128, i128)>,
}

impl AliasTable {
    /// Build alias analysis for one function.
    pub fn analyse(
        function: FunctionId,
        definitions: Arc<DefinitionTable>,
        dominators: Arc<DominatorTable>,
        layouts: Arc<LayoutTable>,
        constants: &ConstantTable,
        target: TargetLayout,
        tree: &Tree,
    ) -> Result<Self, LayoutError> {
        let (regions, operands) = Self::build_regions(
            function,
            &definitions,
            &dominators,
            &layouts,
            constants,
            target,
            tree,
        )?;

        // cache bounds for symbolic address indices
        let mut bounds = FxIndexMap::default();
        for region in regions.iter().flatten().chain(operands.values()) {
            let MemoryRegion::Place(place) = region else {
                continue;
            };
            for offset in &place.indexed_offsets {
                if bounds.contains_key(&offset.index) {
                    continue;
                }
                let ty = tree.get(tree.get(function).expect_value_type(offset.index));
                let Some((width, is_signed)) =
                    ty.int_info_with_pointer_width(target.pointer_bits())
                else {
                    continue;
                };
                let bits = KnownBits::analyse(
                    offset.index,
                    0,
                    tree.get(function),
                    &definitions,
                    target,
                    tree,
                );
                let mask = u128::MAX >> (128 - width);
                let range = if is_signed {
                    Some(bits.signed_range(width))
                } else {
                    i128::try_from(bits.one)
                        .ok()
                        .zip(i128::try_from(!bits.zero & mask).ok())
                };
                if let Some((mut minimum, mut maximum)) = range {
                    if (minimum == 0 || maximum == 0)
                        && offset.index.is_known_nonzero(
                            tree.get(function),
                            &definitions,
                            target,
                            tree,
                        )
                    {
                        if minimum == 0 {
                            minimum = 1;
                        }
                        if maximum == 0 {
                            maximum = -1;
                        }
                    }
                    bounds.insert(offset.index, (minimum, maximum));
                }
            }
        }

        // retain conditional addresses for correlated arm comparisons
        let mut selections = FxIndexMap::default();
        for (value, definition) in definitions.definitions() {
            if let Some(instruction) = definition.instruction()
                && let Instruction::Select {
                    condition,
                    then_value,
                    else_value,
                    ..
                } = tree.get(instruction)
            {
                selections.insert(
                    value,
                    Selection {
                        condition: *condition,
                        values: [*then_value, *else_value],
                    },
                );
            }
        }

        Ok(Self {
            selections,
            bounds,
            target,
            regions,
            operands,
            layouts,
            definitions,
            dominators,
        })
    }

    /// Query whether two memory locations alias.
    pub fn alias(
        &self,
        left: &MemoryLocation,
        right: &MemoryLocation,
    ) -> Result<AliasResult, LayoutError> {
        AliasQuery {
            table: self,
            remaining: QUERY_LIMIT,
            active: SmallVec::new(),
        }
        .alias(
            left,
            right,
            self.address_region(&left.address),
            self.address_region(&right.address),
        )
    }

    /// Compare resolved storage roots and byte offsets.
    fn compare(
        &self,
        left: &MemoryLocation,
        right: &MemoryLocation,
        left_region: &MemoryRegion,
        right_region: &MemoryRegion,
    ) -> Result<AliasResult, LayoutError> {
        let left_size = left.size.byte_len(&self.layouts)?;
        let right_size = right.size.byte_len(&self.layouts)?;

        // exclude accesses that touch no bytes
        if left_size == Some(0) || right_size == Some(0) {
            return Ok(AliasResult::NoAlias);
        }

        // identify equal starting addresses independently of access widths
        if left.address == right.address && left.address.is_stable() {
            return Ok(AliasResult::MustAlias);
        }

        // compare identified storage before opaque regions
        match (left_region, right_region) {
            (MemoryRegion::Place(left_place), MemoryRegion::Place(right_place)) => {
                if left_place.root.is_disjoint_from(&right_place.root) {
                    return Ok(AliasResult::NoAlias);
                }

                // retain offsets selected by runtime dispatch
                if matches!(left.address, MemoryAddress::Dynamic { .. })
                    || matches!(right.address, MemoryAddress::Dynamic { .. })
                {
                    return Ok(AliasResult::MayAlias);
                }

                // compare offsets within the same storage root
                if left_place.root == right_place.root {
                    if !left.address.is_stable() || !right.address.is_stable() {
                        return Ok(AliasResult::MayAlias);
                    }
                    let result = left_place.alias_with(
                        left_size,
                        right_place,
                        right_size,
                        self.target.pointer_bits(),
                    );
                    if result == AliasResult::MayAlias
                        && self.are_ranges_disjoint(left_place, left_size, right_place, right_size)
                    {
                        return Ok(AliasResult::NoAlias);
                    }

                    return Ok(result);
                }
            }
            _ => {
                let left_spaces = left_region.spaces();
                let right_spaces = right_region.spaces();

                // exclude storage spaces that cannot overlap
                if left_spaces.is_disjoint(right_spaces) {
                    return Ok(AliasResult::NoAlias);
                }
            }
        }

        Ok(AliasResult::MayAlias)
    }

    /// Exclude overlap using the interval of the complete relative byte displacement.
    fn are_ranges_disjoint(
        &self,
        left: &MemoryPlace,
        left_size: Option<u64>,
        right: &MemoryPlace,
        right_size: Option<u64>,
    ) -> bool {
        let (Some(left_size), Some(right_size)) = (left_size, right_size) else {
            return false;
        };
        let Some(offset) = left.const_offset.checked_sub(right.const_offset) else {
            return false;
        };
        let mut minimum = offset;
        let mut maximum = offset;

        // accumulate every signed term in the relative offset interval
        for offset in left.difference(right) {
            let Some(&(lower, upper)) = self.bounds.get(&offset.index) else {
                return false;
            };
            let (lower, upper) = if offset.scale < 0 {
                (upper, lower)
            } else {
                (lower, upper)
            };
            let lower = lower
                .checked_mul(offset.scale)
                .and_then(|value| minimum.checked_add(value));
            let upper = upper
                .checked_mul(offset.scale)
                .and_then(|value| maximum.checked_add(value));
            let (Some(lower), Some(upper)) = (lower, upper) else {
                return false;
            };
            minimum = lower;
            maximum = upper;
        }

        // require the interval to remain between access widths within one address period
        let modulus = 1i128 << self.target.pointer_bits();
        minimum.div_euclid(modulus) == maximum.div_euclid(modulus)
            && minimum.rem_euclid(modulus) >= i128::from(right_size)
            && modulus - maximum.rem_euclid(modulus) >= i128::from(left_size)
    }

    /// Return true when two address values may alias.
    pub fn addresses_may_alias(&self, left: Value, right: Value) -> Result<bool, LayoutError> {
        let left = MemoryLocation::from_address(left);
        let right = MemoryLocation::from_address(right);

        Ok(self.alias(&left, &right)?.may_alias())
    }

    /// Return true when two address values definitely do not alias.
    pub fn addresses_no_alias(&self, left: Value, right: Value) -> Result<bool, LayoutError> {
        let left = MemoryLocation::from_address(left);
        let right = MemoryLocation::from_address(right);

        Ok(self.alias(&left, &right)?.is_no_alias())
    }

    /// Return whether an addressed location may touch one storage root.
    pub fn may_touch_root(
        &self,
        location: &MemoryLocation,
        storage: &StorageRoot,
    ) -> Result<bool, LayoutError> {
        if location.size.byte_len(&self.layouts)? == Some(0) {
            return Ok(false);
        }
        Ok(self
            .address_region(&location.address)
            .may_touch_root(storage))
    }

    /// Return the physical region of a complete memory operand.
    fn address_region(&self, address: &MemoryAddress) -> &MemoryRegion {
        match address {
            MemoryAddress::Dynamic { value, .. } => self.region(*value),
            MemoryAddress::Place(place) => {
                if let PlaceOrigin::Value(value) = place.origin
                    && place.path.projections == [Projection::Deref]
                {
                    return self.region(value);
                }

                self.operands
                    .get(place)
                    .unwrap_or_else(|| unreachable!("unrecorded memory operand {place:?}"))
            }
        }
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

    /// Return whether a queried region denotes the same addresses across a merge.
    pub fn is_invariant(&self, region: &MemoryRegion, block: BlockId) -> bool {
        let region = match region {
            MemoryRegion::Address { location, .. } => {
                if matches!(location.address, MemoryAddress::Dynamic { .. }) {
                    return location
                        .address
                        .value()
                        .is_some_and(|value| self.is_available(value, block));
                }
                let resolved = self.address_region(&location.address);
                if matches!(resolved, MemoryRegion::Any { .. }) {
                    return location
                        .address
                        .value()
                        .is_some_and(|value| self.is_available(value, block));
                }

                resolved
            }
            region => region,
        };

        // keep activation storage and incoming parameters stable across merges
        if let MemoryRegion::Place(place) = region {
            if let StorageRoot::Address { place, .. } = &place.root
                && (!place.is_stable()
                    || place
                        .uses()
                        .iter()
                        .any(|value| !self.is_available(*value, block)))
            {
                return false;
            }
            let allocation = match place.root {
                StorageRoot::Allocation { block, .. } => Some(block),
                _ => None,
            };
            if allocation.is_some_and(|definition| {
                definition == block || !self.dominators.dominates(definition, block)
            }) {
                return false;
            }

            return place
                .indexed_offsets
                .iter()
                .all(|offset| self.is_available(offset.index, block));
        }

        true
    }

    /// Return whether one address component precedes a block entry.
    fn is_available(&self, value: Value, block: BlockId) -> bool {
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

    /// Build all memory regions for one function.
    fn build_regions(
        function: FunctionId,
        definitions: &DefinitionTable,
        dominators: &DominatorTable,
        layouts: &LayoutTable,
        constants: &ConstantTable,
        target: TargetLayout,
        tree: &Tree,
    ) -> Result<(Vec<Option<MemoryRegion>>, FxIndexMap<Place, MemoryRegion>), LayoutError> {
        let mut builder = MemoryRegionBuilder::new(
            function,
            definitions,
            dominators,
            tree,
            layouts,
            constants,
            target,
        );

        // allocate one region entry per SSA value
        let mut regions = vec![None; builder.tree.get(function).value_capacity()];

        // resolve each defined SSA value once
        for (value, _) in definitions.definitions() {
            regions[value.id() as usize] = Some(builder.region(value, 0)?);
        }

        // resolve direct operands and reference prefixes read along their paths
        let mut operands = FxIndexMap::default();
        for block_index in 0..builder.tree.get(function).blocks().len() {
            let block = builder.tree.get(function).blocks()[block_index];
            for index in 0..builder.tree.get(block).instructions.len() {
                let instruction = builder.tree.get(block).instructions[index];
                let instruction = builder.tree.get(instruction).clone();
                if let Some(place) = instruction.place() {
                    let mut prefix = Place::new(place.origin);
                    for projection in &place.path.projections {
                        if *projection == Projection::Deref && !operands.contains_key(&prefix) {
                            operands.insert(prefix.clone(), builder.place(&prefix, 0)?);
                        }
                        prefix.push(projection.clone());
                    }
                    if !operands.contains_key(place) {
                        operands.insert(place.clone(), builder.place(place, 0)?);
                    }
                }
            }
        }

        Ok((regions, operands))
    }
}

/// One address selected by a condition.
#[derive(Debug)]
struct Selection {
    /// The condition selecting an arm.
    condition: Value,
    /// Addresses selected by true and false respectively.
    values: [Value; 2],
}

/// Recursive alias comparisons for one pair of memory accesses.
struct AliasQuery<'a> {
    /// Resolved addresses and incoming definitions.
    table: &'a AliasTable,
    /// The comparisons still permitted by this query.
    remaining: usize,
    /// Pairs being compared, used to resolve recursive phi cycles.
    active: SmallVec<[(MemoryAddress, MemorySize, MemoryAddress, MemorySize); 8]>,
}

impl AliasQuery<'_> {
    /// Compare locations and expand unresolved conditional addresses.
    fn alias(
        &mut self,
        left: &MemoryLocation,
        right: &MemoryLocation,
        left_region: &MemoryRegion,
        right_region: &MemoryRegion,
    ) -> Result<AliasResult, LayoutError> {
        // preserve resolved relationships and runtime projections
        let result = self.table.compare(left, right, left_region, right_region)?;
        if result != AliasResult::MayAlias
            || matches!(left.address, MemoryAddress::Dynamic { .. })
            || matches!(right.address, MemoryAddress::Dynamic { .. })
        {
            return Ok(result);
        }

        // require every nonrecursive incoming pair to establish disjointness
        let pair = (
            left.address.clone(),
            left.size,
            right.address.clone(),
            right.size,
        );
        if self.active.contains(&pair)
            || self
                .active
                .contains(&(pair.2.clone(), pair.3, pair.0.clone(), pair.1))
        {
            return Ok(AliasResult::NoAlias);
        }
        if self.remaining == 0 {
            return Ok(AliasResult::MayAlias);
        }
        self.remaining -= 1;
        self.active.push(pair);

        // remove the active pair even when a required layout is absent
        let result = self.compare(left, right, left_region, right_region);
        self.active.pop();

        result
    }

    /// Substitute a conditional reference while preserving the operand's byte displacement.
    fn substitute(
        &self,
        location: &mut MemoryLocation,
        region: &MemoryRegion,
        from: Value,
        to: Value,
    ) -> MemoryRegion {
        if let MemoryAddress::Place(place) = &mut location.address {
            place.map_values(|value| if value == from { to } else { value });
        }

        let mut region = region.clone();
        if let MemoryRegion::Place(projected) = &mut region
            && let StorageRoot::Address { place, .. } = &mut projected.root
            && place.origin == PlaceOrigin::Value(from)
        {
            if place.path.projections == [Projection::Deref] {
                let mut base = self.table.region(to).clone();
                if let MemoryRegion::Place(base) = &mut base {
                    base.const_offset = base.const_offset.wrapping_add(projected.const_offset);
                    for offset in &projected.indexed_offsets {
                        base.add_indexed_offset(offset.index, offset.scale);
                    }
                }

                return base;
            }
            place.map_values(|value| if value == from { to } else { value });
        }

        region
    }

    /// Compare matching select arms and phi edges before independent alternatives.
    fn compare(
        &mut self,
        left: &MemoryLocation,
        right: &MemoryLocation,
        left_region: &MemoryRegion,
        right_region: &MemoryRegion,
    ) -> Result<AliasResult, LayoutError> {
        let table = self.table;
        let (Some(left_value), Some(right_value)) = (left.address.value(), right.address.value())
        else {
            return Ok(AliasResult::MayAlias);
        };

        // correlate select arms guarded by the same SSA condition
        if let Some(selection) = table.selections.get(&left_value) {
            let other = table.selections.get(&right_value);
            let mut result = None;
            for (index, &value) in selection.values.iter().enumerate() {
                let mut left = left.clone();
                let mut right = right.clone();
                let left_region = self.substitute(&mut left, left_region, left_value, value);
                let mut right_region = right_region.clone();
                if let Some(other) = other.filter(|other| other.condition == selection.condition) {
                    right_region = self.substitute(
                        &mut right,
                        &right_region,
                        right_value,
                        other.values[index],
                    );
                }
                let alias = self.alias(&left, &right, &left_region, &right_region)?;
                result = Some(result.map_or(alias, |previous| {
                    if previous == alias {
                        alias
                    } else {
                        AliasResult::MayAlias
                    }
                }));
            }

            return Ok(result.unwrap_or(AliasResult::MayAlias));
        }
        if table.selections.contains_key(&right_value) {
            return self.compare(right, left, right_region, left_region);
        }

        // compare matching incoming edges of parameters declared in the same block
        let definition = table.definitions.definition(left_value);
        if let Some(ValueDefinition::BlockParameter { block, .. }) = definition {
            let is_paired = match table.definitions.definition(right_value) {
                Some(ValueDefinition::BlockParameter { block: other, .. }) => other == block,
                _ => false,
            };
            if !is_paired && !table.is_invariant(right_region, block) {
                return Ok(AliasResult::MayAlias);
            }

            // merge alias results from matching incoming edges
            let mut result = None;
            for input in table.definitions.inputs(left_value) {
                let Some(argument) = input.argument else {
                    return Ok(AliasResult::MayAlias);
                };
                let mut left = left.clone();
                let mut right = right.clone();
                let left_region = self.substitute(&mut left, left_region, left_value, argument);
                let mut right_region = right_region.clone();
                if is_paired {
                    let other = table
                        .definitions
                        .inputs(right_value)
                        .iter()
                        .find(|other| other.edge == input.edge)
                        .unwrap_or_else(|| {
                            unreachable!("block parameters have different incoming edges")
                        });
                    let Some(argument) = other.argument else {
                        return Ok(AliasResult::MayAlias);
                    };
                    right_region =
                        self.substitute(&mut right, &right_region, right_value, argument);
                }
                let alias = self.alias(&left, &right, &left_region, &right_region)?;
                result = Some(result.map_or(alias, |previous| {
                    if previous == alias {
                        alias
                    } else {
                        AliasResult::MayAlias
                    }
                }));
                if result == Some(AliasResult::MayAlias) {
                    break;
                }
            }

            return Ok(result.unwrap_or(AliasResult::MayAlias));
        }
        if matches!(
            table.definitions.definition(right_value),
            Some(ValueDefinition::BlockParameter { .. })
        ) {
            return self.compare(right, left, right_region, left_region);
        }

        Ok(AliasResult::MayAlias)
    }
}

impl Analysis for AliasTable {
    const INVALIDATED_BY: Mutation = Mutation::CONTROL
        .union(Mutation::VALUE)
        .union(Mutation::LAYOUT);
}

/// Result of an alias query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AliasResult {
    /// The locations start at the same address.
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
    use crate::Substitution;

    use crate::analyses::tests::TestModule;

    /// Independent local slots do not alias.
    #[test]
    fn test_separate_distinct_local_storage() {
        let program = TestModule::new(
            r#"
function test(): int32 {
    local l0: int32
    local l1: int32

entry:
    v0: ref<int32, borrowed, 'frame, mutable, frame> = address l0
    v1: ref<int32, borrowed, 'frame, mutable, frame> = address l1
    v2: int32 = 0
    return v2
}
"#,
        );

        let function_id = program.entry_function_id();
        let mut analyses = program.function_analyses();
        let alias = analyses
            .alias(function_id, program.layouts.clone(), &program.tree)
            .unwrap();
        let addresses = program.local_address_destinations_in_entry(function_id);

        // compare two distinct storage roots
        let left = MemoryLocation::from_address(addresses[0]);
        let right = MemoryLocation::from_address(addresses[1]);

        assert_eq!(alias.alias(&left, &right).unwrap(), AliasResult::NoAlias);
    }

    /// Classify equal addresses independently of their access widths.
    #[test]
    fn test_match_equal_addresses_and_exclude_empty_accesses() {
        let program = TestModule::new(
            r#"
function test(): void {
    local l0: int64

entry:
    v0: ref<int64, borrowed, 'frame, mutable, frame> = address l0
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &program.tree,
            )
            .unwrap();
        let address = Value::new(0);

        for (left, right, expected) in [
            (Some(8), Some(8), AliasResult::MustAlias),
            (Some(4), Some(8), AliasResult::MustAlias),
            (None, Some(8), AliasResult::MustAlias),
            (None, None, AliasResult::MustAlias),
            (Some(0), Some(8), AliasResult::NoAlias),
            (Some(8), Some(0), AliasResult::NoAlias),
        ] {
            let mut left_location = MemoryLocation::from_address(address);
            left_location.size = left.into();
            let mut right_location = MemoryLocation::from_address(address);
            right_location.size = right.into();

            assert_eq!(
                alias.alias(&left_location, &right_location).unwrap(),
                expected
            );
        }
    }

    /// Preserve possible aliasing between independent borrowed parameters.
    #[test]
    fn test_allow_aliasing_between_borrowed_parameters() {
        let program = TestModule::new(
            r#"
function test<'a>(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'a, mutable, local>): void {
entry(v0: ref<int32, borrowed, 'a, mutable, local>, v1: ref<int32, borrowed, 'a, mutable, local>):
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &program.tree,
            )
            .unwrap();
        let left = MemoryLocation::with_size(Value::new(0), 4);
        let right = MemoryLocation::with_size(Value::new(1), 4);

        assert_eq!(alias.alias(&left, &right).unwrap(), AliasResult::MayAlias);
    }

    /// Preserve possible aliasing after storing a selected reference in two locals.
    #[test]
    fn test_allow_aliasing_between_reloaded_references() {
        let program = TestModule::new(
            r#"
function test<'a>(v0: boolean, v1: ref<int32, borrowed, 'a, mutable, local>, v2: ref<int32, borrowed, 'a, mutable, local>): void {
    local l0: ref<int32, borrowed, 'a, mutable, local>
    local l1: ref<int32, borrowed, 'a, mutable, local>

entry(v0: boolean, v1: ref<int32, borrowed, 'a, mutable, local>, v2: ref<int32, borrowed, 'a, mutable, local>):
    v3: ref<int32, borrowed, 'a, mutable, local> = select v0, v1, v2
    store l0, v3
    store l1, v3
    v4: ref<int32, borrowed, 'a, mutable, local> = load l0
    v5: ref<int32, borrowed, 'a, mutable, local> = load l1
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &program.tree,
            )
            .unwrap();
        let left = MemoryLocation::with_size(Value::new(4), 4);
        let right = MemoryLocation::with_size(Value::new(5), 4);

        assert_eq!(alias.alias(&left, &right).unwrap(), AliasResult::MayAlias);
    }

    /// Preserve pointer bitcasts and widen addresses reconstructed from integers.
    #[test]
    fn test_preserve_pointer_casts_and_widen_integer_reconstruction() {
        let program = TestModule::new(
            r#"
function test(): void {
    local l0: int32
    local l1: int32

entry:
    v0: ref<int32, borrowed, 'frame, mutable, frame> = address l0
    v1: ref<int32, borrowed, 'frame, mutable, frame> = address l1
    v2: ptr<int32, mutable> = cast.bit v0 -> ptr<int32, mutable>
    v3: usize = cast.pointerToInt v0 -> usize
    v4: uint8 = cast.intToInt v3 -> uint8
    v5: ptr<int32, mutable> = cast.intToPointer v4 -> ptr<int32, mutable>
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &program.tree,
            )
            .unwrap();

        for (left, right, expected) in [
            (0, 2, AliasResult::MustAlias),
            (1, 2, AliasResult::NoAlias),
            (0, 5, AliasResult::MayAlias),
            (1, 5, AliasResult::MayAlias),
        ] {
            let left = MemoryLocation::with_size(Value::new(left), 4);
            let right = MemoryLocation::with_size(Value::new(right), 4);

            assert_eq!(alias.alias(&left, &right).unwrap(), expected);
            assert_eq!(alias.alias(&right, &left).unwrap(), expected);
        }
    }

    /// Compare accessed bytes across adjacent aggregate fields.
    #[test]
    fn test_compare_field_extents() {
        let program = TestModule::new(
            r#"
type Pair {
    first: int32;
    second: int32;
}

function test<'a>(v0: ref<Pair, borrowed, 'a, mutable, local>): void {
entry(v0: ref<Pair, borrowed, 'a, mutable, local>):
    v1: ref<int32, borrowed, 'a, mutable, local> = address (*v0).0
    v2: ref<int32, borrowed, 'a, mutable, local> = address (*v0).1
    v3: int32 = load (*v0).0
    v4: int32 = load (*v0).1
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &program.tree,
            )
            .unwrap();
        let addresses: [(MemoryAddress, MemoryAddress); 2] = [
            (Value(1).into(), Value(2).into()),
            (
                Place::value(Value(0))
                    .with_projection(Projection::Deref)
                    .with_projection(Projection::Field { index: 0 })
                    .into(),
                Place::value(Value(0))
                    .with_projection(Projection::Deref)
                    .with_projection(Projection::Field { index: 1 })
                    .into(),
            ),
        ];

        for (size, expected) in [
            (Some(4), AliasResult::NoAlias),
            (Some(8), AliasResult::PartialAlias),
            (None, AliasResult::MayAlias),
            (Some(0), AliasResult::NoAlias),
        ] {
            // compare explicit operands and their materialized addresses identically
            for (first, second) in &addresses {
                let mut first = MemoryLocation::from_address(first.clone());
                first.size = size.into();
                let second = MemoryLocation::with_size(second.clone(), 4);

                assert_eq!(alias.alias(&first, &second).unwrap(), expected);
                assert_eq!(alias.alias(&second, &first).unwrap(), expected);
            }
        }
    }

    /// Preserve overlap between alternative variant payloads stored at the same offset.
    #[test]
    fn test_match_variant_payloads_at_the_same_offset() {
        let program = TestModule::new(
            r#"
type Choice = variant<uint1> { 0uint1 = int32; 1uint1 = uint32; };

function test<'a>(v0: ref<Choice, borrowed, 'a, mutable, local>): void {
entry(v0: ref<Choice, borrowed, 'a, mutable, local>):
    v1: ptr<int32, mutable> = address ((*v0) as 0)
    v2: ptr<uint32, mutable> = address ((*v0) as 1)
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &program.tree,
            )
            .unwrap();
        let first = MemoryLocation::with_size(Value::new(1), 4);
        let second = MemoryLocation::with_size(Value::new(2), 4);

        assert_eq!(
            alias.alias(&first, &second).unwrap(),
            AliasResult::MustAlias
        );
    }

    /// Resolve constant element indices and their canonical strides.
    #[test]
    fn test_distinguish_adjacent_and_overlapping_elements() {
        let program = TestModule::new(
            r#"
function test<'a>(v0: ref<[int32; 4], borrowed, 'a, mutable, local>): void {
entry(v0: ref<[int32; 4], borrowed, 'a, mutable, local>):
    v1: usize = 0
    v2: usize = 1
    v3: ref<int32, borrowed, 'a, mutable, local> = address (*v0)[v1]
    v4: ref<int32, borrowed, 'a, mutable, local> = address (*v0)[v2]
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &program.tree,
            )
            .unwrap();
        let first = MemoryLocation::with_size(Value::new(3), 4);
        let second = MemoryLocation::with_size(Value::new(4), 4);
        let overlapping = MemoryLocation::with_size(Value::new(3), 8);

        assert_eq!(alias.alias(&first, &second).unwrap(), AliasResult::NoAlias);
        assert_eq!(
            alias.alias(&overlapping, &second).unwrap(),
            AliasResult::PartialAlias
        );
    }

    /// Preserve stable addresses through selected values and block parameters.
    #[test]
    fn test_match_common_addresses_after_merges() {
        let program = TestModule::new(
            r#"
function test(v0: boolean): void {
    local l0: int32

entry(v0: boolean):
    v1: ptr<int32, mutable> = address l0
    v2: ptr<int32, mutable> = address l0
    v3: ptr<int32, mutable> = select v0, v1, v2
    branch v0 => join(v1) | join(v3)

join(v4: ptr<int32, mutable>):
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &program.tree,
            )
            .unwrap();
        let original = MemoryLocation::with_size(Value(1), 4);
        let merged = MemoryLocation::with_size(Value(4), 4);

        assert_eq!(
            alias.alias(&original, &merged).unwrap(),
            AliasResult::MustAlias
        );
    }

    /// Treat both fresh loop allocations and incoming allocation parameters as varying addresses.
    #[test]
    fn test_detect_allocations_that_change_between_iterations() {
        let program = TestModule::new(
            r#"
function test(v0: ref<int32, unique, mutable, local>, v1: boolean): void {
entry(v0: ref<int32, unique, mutable, local>, v1: boolean):
    jump loop(v0)

loop(v2: ref<int32, unique, mutable, local>):
    v3: ref<int32, unique, mutable, local> = new.zeroed int32
    branch v1 => loop(v3) | exit

exit:
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &program.tree,
            )
            .unwrap();
        let block = program.tree.get(program.entry_function_id()).block(1);
        assert!(!alias.is_invariant(alias.region(Value(2)), block));
        assert!(!alias.is_invariant(alias.region(Value(3)), block));
    }

    /// Require the canonical layout when projecting a field address.
    #[test]
    fn test_require_field_layout() {
        let program = TestModule::new(
            r#"
type Pair {
    first: int32;
    second: int32;
}

function test(v0: ptr<Pair, mutable>): void {
entry(v0: ptr<Pair, mutable>):
    v1: ptr<int32, mutable> = address (*v0).1
    return
}
"#,
        );
        let ty = program
            .tree
            .get(program.entry_function_id())
            .expect_value_type(Value(0));
        let ty = Substitution::resolve(ty, &program.tree);
        let ty = program.tree.get(ty).pointee_type().unwrap();
        let mut analyses = program.function_analyses();
        let error = analyses
            .alias(
                program.entry_function_id(),
                Arc::new(LayoutTable::new()),
                &program.tree,
            )
            .unwrap_err();

        assert_eq!(error, LayoutError::Missing { ty });
    }

    /// Keep changing entry parameters distinct from invariant incoming addresses.
    #[test]
    fn test_track_entry_backedge_addresses() {
        let program = TestModule::new(
            r#"
function test(v0: ref<int32, unique, mutable, local>, v1: ref<int32, borrowed, 'static, readonly, local>, v2: boolean): void {
entry(v0: ref<int32, unique, mutable, local>, v1: ref<int32, borrowed, 'static, readonly, local>, v2: boolean):
    v3: ref<int32, unique, mutable, local> = new.zeroed int32
    branch v2 => entry(v3, v1, v2) | done

done:
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &program.tree,
            )
            .unwrap();
        let actual = [Value(0), Value(1)].map(|value| {
            let region = MemoryRegion::from_address(value, None, None, None);

            alias.is_invariant(
                &region,
                program.tree.get(program.entry_function_id()).block(0),
            )
        });

        assert_eq!(actual, [false, true]);
    }

    /// Compare matching select arms while preserving independent conditions.
    #[test]
    fn test_correlate_addresses_selected_by_the_same_condition() {
        let program = TestModule::new(
            r#"
function test(v0: boolean, v1: boolean): void {
    local l0: int32
    local l1: int32

entry(v0: boolean, v1: boolean):
    v2: ref<int32, borrowed, 'frame, mutable, frame> = address l0
    v3: ref<int32, borrowed, 'frame, mutable, frame> = address l1
    v4: ref<int32, borrowed, 'frame, mutable, frame> = select v0, v2, v3
    v5: ref<int32, borrowed, 'frame, mutable, frame> = select v0, v3, v2
    v6: ref<int32, borrowed, 'frame, mutable, frame> = select v0, v2, v3
    v7: ref<int32, borrowed, 'frame, mutable, frame> = select v1, v2, v3
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &program.tree,
            )
            .unwrap();
        let left = MemoryLocation::with_size(Value(4), 4);
        let actual = [5, 6, 7].map(|value| {
            alias
                .alias(&left, &MemoryLocation::with_size(Value(value), 4))
                .unwrap()
        });

        assert_eq!(
            actual,
            [
                AliasResult::NoAlias,
                AliasResult::MustAlias,
                AliasResult::MayAlias
            ]
        );
    }

    /// Preserve disjointness and access widths when loop parameters exchange addresses.
    #[test]
    fn test_preserve_disjoint_ranges_when_loop_arguments_swap() {
        let program = TestModule::new(
            r#"
function test(v0: boolean): void {
    local l0: int64
    local l1: int64

entry(v0: boolean):
    v1: ref<int64, borrowed, 'frame, mutable, frame> = address l0
    v2: ref<int64, borrowed, 'frame, mutable, frame> = address l1
    jump loop(v1, v2)

loop(v3: ref<int64, borrowed, 'frame, mutable, frame>, v4: ref<int64, borrowed, 'frame, mutable, frame>):
    branch v0 => loop(v4, v3) | done

done:
    return
}

function overlap(v0: boolean): void {
    local l0: [int32; 3]

entry(v0: boolean):
    v1: ref<[int32; 3], borrowed, 'frame, mutable, frame> = address l0
    v2: int64 = 0
    v3: int64 = 1
    v4: ref<int32, borrowed, 'frame, mutable, frame> = address (*v1)[v2]
    v5: ref<int32, borrowed, 'frame, mutable, frame> = address (*v1)[v3]
    jump loop(v4, v5)

loop(v6: ref<int32, borrowed, 'frame, mutable, frame>, v7: ref<int32, borrowed, 'frame, mutable, frame>):
    branch v0 => loop(v7, v6) | done

done:
    return
}
"#,
        );
        for (name, left, right, expected) in [
            ("test", Value(3), Value(4), AliasResult::NoAlias),
            ("overlap", Value(6), Value(7), AliasResult::MayAlias),
        ] {
            let mut analyses = program.function_analyses();
            let alias = analyses
                .alias(
                    program.function_id_by_name(name),
                    program.layouts.clone(),
                    &program.tree,
                )
                .unwrap();
            let actual = alias
                .alias(
                    &MemoryLocation::with_size(left, 4),
                    &MemoryLocation::with_size(right, 8),
                )
                .unwrap();

            assert_eq!(actual, expected, "{name}");
        }
    }

    /// Retain field offsets through loaded pointers without separating their allocations.
    #[test]
    fn test_compare_fields_of_loaded_addresses() {
        let program = TestModule::new(
            r#"
type Pair {
    first: int32;
    second: int32;
}

function test(v0: ptr<ptr<Pair, mutable>, mutable>): void {
entry(v0: ptr<ptr<Pair, mutable>, mutable>):
    v1: ptr<Pair, mutable> = load (*v0)
    v2: ptr<Pair, mutable> = load (*v0)
    v3: ptr<int32, mutable> = address (*v1).0
    v4: ptr<int32, mutable> = address (*v1).1
    v5: ptr<int32, mutable> = address (*v2).1
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &program.tree,
            )
            .unwrap();
        let actual = [(3, 4), (3, 5), (4, 5)].map(|(left, right)| {
            alias
                .alias(
                    &MemoryLocation::with_size(Value(left), 4),
                    &MemoryLocation::with_size(Value(right), 4),
                )
                .unwrap()
        });

        assert_eq!(
            actual,
            [
                AliasResult::NoAlias,
                AliasResult::MayAlias,
                AliasResult::MayAlias
            ]
        );
    }

    /// Separate adjacent bounded indices while retaining potentially wrapping expressions.
    #[test]
    fn test_separate_bounded_indices_and_allow_wrapping_overlap() {
        let program = TestModule::new(
            r#"
function test<'a>(v0: ref<[int32; 256], borrowed, 'a, mutable, local>, v1: uint8): void {
entry(v0: ref<[int32; 256], borrowed, 'a, mutable, local>, v1: uint8):
    v2: uint8 = 63
    v3: uint8 = 1
    v4: uint8 = and v1, v2
    v5: uint8 = add v4, v3
    v6: ref<int32, borrowed, 'a, mutable, local> = address (*v0)[v4]
    v7: ref<int32, borrowed, 'a, mutable, local> = address (*v0)[v5]
    v8: uint8 = add v1, v3
    v9: ref<int32, borrowed, 'a, mutable, local> = address (*v0)[v8]
    v10: uint8 = intrinsic.math.arithmetic.unchecked.add(v4, v3)
    v11: ref<int32, borrowed, 'a, mutable, local> = address (*v0)[v10]
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &program.tree,
            )
            .unwrap();
        let actual = [(6, 7), (7, 11), (6, 9)].map(|(left, right)| {
            alias
                .alias(
                    &MemoryLocation::with_size(Value(left), 4),
                    &MemoryLocation::with_size(Value(right), 4),
                )
                .unwrap()
        });

        assert_eq!(
            actual,
            [
                AliasResult::NoAlias,
                AliasResult::MustAlias,
                AliasResult::MayAlias
            ]
        );
    }

    /// Separate fields at different residues across arbitrary array elements.
    #[test]
    fn test_separate_fields_across_arbitrary_array_indices() {
        let program = TestModule::new(
            r#"
type Pair {
    first: int32;
    second: int32;
}

function test<'a>(v0: ref<[Pair; 16], borrowed, 'a, mutable, local>, v1: uint64, v2: uint64): void {
entry(v0: ref<[Pair; 16], borrowed, 'a, mutable, local>, v1: uint64, v2: uint64):
    v3: ref<Pair, borrowed, 'a, mutable, local> = address (*v0)[v1]
    v4: ref<Pair, borrowed, 'a, mutable, local> = address (*v0)[v2]
    v5: ref<int32, borrowed, 'a, mutable, local> = address (*v3).0
    v6: ref<int32, borrowed, 'a, mutable, local> = address (*v4).1
    v7: ref<int32, borrowed, 'a, mutable, local> = address (*v4).0
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &program.tree,
            )
            .unwrap();
        let actual = [(5, 6, 4), (5, 7, 4), (5, 6, 8)].map(|(left, right, width)| {
            alias
                .alias(
                    &MemoryLocation::with_size(Value(left), width),
                    &MemoryLocation::with_size(Value(right), 4),
                )
                .unwrap()
        });

        assert_eq!(
            actual,
            [
                AliasResult::NoAlias,
                AliasResult::MayAlias,
                AliasResult::MayAlias
            ]
        );
    }

    /// Compare byte displacements modulo the target pointer width.
    #[test]
    fn test_compare_offsets_modulo_pointer_width() {
        let program = TestModule::new(
            r#"
function test(v0: ptr<[uint8; 1], mutable>): void {
entry(v0: ptr<[uint8; 1], mutable>):
    v1: uint128 = 0
    v2: uint128 = 18446744073709551616
    v3: ptr<uint8, mutable> = address (*v0)[v1]
    v4: ptr<uint8, mutable> = address (*v0)[v2]
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &program.tree,
            )
            .unwrap();
        let result = alias
            .alias(
                &MemoryLocation::with_size(Value(3), 1),
                &MemoryLocation::with_size(Value(4), 1),
            )
            .unwrap();

        assert_eq!(result, AliasResult::MustAlias);
    }

    /// Separate bounded index intervals and cancel multiplication with equivalent shifts.
    #[test]
    fn test_separate_index_intervals_and_cancel_equivalent_scaling() {
        let program = TestModule::new(
            r#"
function test<'a>(v0: ref<[int32; 256], borrowed, 'a, mutable, local>, v1: uint8): void {
entry(v0: ref<[int32; 256], borrowed, 'a, mutable, local>, v1: uint8):
    v2: uint8 = 15
    v3: uint8 = 16
    v4: uint8 = 2
    v5: uint8 = 1
    v6: uint8 = and v1, v2
    v7: uint8 = or v6, v3
    v8: uint8 = mul v6, v4
    v9: uint8 = shl v6, v5
    v10: ref<int32, borrowed, 'a, mutable, local> = address (*v0)[v6]
    v11: ref<int32, borrowed, 'a, mutable, local> = address (*v0)[v7]
    v12: ref<int32, borrowed, 'a, mutable, local> = address (*v0)[v8]
    v13: ref<int32, borrowed, 'a, mutable, local> = address (*v0)[v9]
    v14: uint8 = 0
    v15: ref<int32, borrowed, 'a, mutable, local> = address (*v0)[v14]
    return
}
"#,
        );
        let mut analyses = program.function_analyses();
        let alias = analyses
            .alias(
                program.entry_function_id(),
                program.layouts.clone(),
                &program.tree,
            )
            .unwrap();
        let actual = [(10, 11), (12, 13), (11, 15), (10, 15)].map(|(left, right)| {
            alias
                .alias(
                    &MemoryLocation::with_size(Value(left), 4),
                    &MemoryLocation::with_size(Value(right), 4),
                )
                .unwrap()
        });

        assert_eq!(
            actual,
            [
                AliasResult::NoAlias,
                AliasResult::MustAlias,
                AliasResult::NoAlias,
                AliasResult::MayAlias
            ]
        );
    }
}
