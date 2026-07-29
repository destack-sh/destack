use crate as mir;

use crate::{
    Analysis, AnalysisId, FunctionAnalysis, FunctionAnalysisCache, MemoryRegion,
    MemoryRegionBuilder, ReferenceLocation, StorageRoot, TargetLayout, ValueDefinitions,
    ValueTypes,
};

/// Alias analysis for one MIR function.
#[derive(Debug)]
pub struct AliasAnalysis {
    /// Memory region indexed by SSA value id.
    regions: Vec<Option<MemoryRegion>>,
}

impl AliasAnalysis {
    /// Build alias analysis for one function.
    pub fn build(
        function: &mir::Function,
        definitions: &ValueDefinitions,
        value_types: &ValueTypes,
        target_layout: TargetLayout,
        tree: &mir::Tree,
    ) -> Self {
        let regions = Self::build_regions(function, definitions, value_types, target_layout, tree);

        Self { regions }
    }

    /// Query whether two memory locations alias.
    pub fn alias(&self, left: &ReferenceLocation, right: &ReferenceLocation) -> AliasResult {
        if left.reference == right.reference {
            return left.alias_same_reference(right);
        }

        let left_region = self.region(left.reference);
        let right_region = self.region(right.reference);

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

    /// Return true when two reference values may alias.
    pub fn references_may_alias(&self, left: mir::Value, right: mir::Value) -> bool {
        let left = ReferenceLocation::from_reference(left);
        let right = ReferenceLocation::from_reference(right);

        self.alias(&left, &right).may_alias()
    }

    /// Return true when two reference values definitely do not alias.
    pub fn references_no_alias(&self, left: mir::Value, right: mir::Value) -> bool {
        let left = ReferenceLocation::from_reference(left);
        let right = ReferenceLocation::from_reference(right);

        self.alias(&left, &right).is_no_alias()
    }

    /// Return whether a reference location may touch one storage root.
    pub fn may_touch_root(&self, location: &ReferenceLocation, storage: &StorageRoot) -> bool {
        self.region(location.reference).may_touch_root(storage)
    }

    /// Resolve one reference value into a memory region.
    fn region(&self, reference: mir::Value) -> &MemoryRegion {
        let Some(region) = self
            .regions
            .get(reference.0 as usize)
            .and_then(Option::as_ref)
        else {
            panic!("missing memory region for reference value: {reference:?}");
        };

        region
    }

    /// Build all memory regions for one function.
    fn build_regions(
        function: &mir::Function,
        definitions: &ValueDefinitions,
        value_types: &ValueTypes,
        target_layout: TargetLayout,
        tree: &mir::Tree,
    ) -> Vec<Option<MemoryRegion>> {
        let mut builder = MemoryRegionBuilder::new(
            definitions,
            tree,
            &function.parameters,
            value_types,
            target_layout,
        );

        let mut regions = vec![None; function.value_capacity()];

        // resolve each defined SSA value once
        for (value, _) in definitions.definitions() {
            regions[value.0 as usize] = Some(builder.region(value));
        }

        regions
    }
}

impl Analysis for AliasAnalysis {
    const ID: AnalysisId = AnalysisId("alias");
    const INVALIDATED_BY: mir::Mutation = mir::Mutation::VALUE.union(mir::Mutation::MEMORY);
}

impl FunctionAnalysis for AliasAnalysis {
    fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        analyses: &FunctionAnalysisCache,
    ) -> Self {
        let definitions = analyses.get::<ValueDefinitions>(function, tree);
        let value_types = analyses.get::<ValueTypes>(function, tree);

        Self::build(
            function,
            &definitions,
            &value_types,
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
    /// Check whether this proves non-aliasing.
    pub fn is_no_alias(self) -> bool {
        matches!(self, AliasResult::NoAlias)
    }

    /// Check whether the locations may alias.
    pub fn may_alias(self) -> bool {
        !matches!(self, AliasResult::NoAlias)
    }

    /// Check whether this proves identical memory.
    pub fn is_must_alias(self) -> bool {
        matches!(self, AliasResult::MustAlias)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    v0: ref<int32, raw, mutable, frame> = local.address l0
    v1: ref<int32, raw, mutable, frame> = local.address l1
    v2: int32 = 0
    return v2
}
"#,
        );

        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let analyses = program.function_analysis_cache();
        let alias = analyses.get::<AliasAnalysis>(function, &program.tree);
        let addresses = program.local_address_destinations_in_entry(function_id);

        // compare two distinct storage roots
        let left = ReferenceLocation::from_reference(addresses[0]);
        let right = ReferenceLocation::from_reference(addresses[1]);

        assert_eq!(alias.alias(&left, &right), AliasResult::NoAlias);
    }
}
