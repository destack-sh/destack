use std::sync::Arc;

use crate as mir;
use crate::{
    AliasTable, Analysis, ConstantTable, ControlTable, DefinitionTable, DominatorTable,
    InitializationTable, LivenessTable, LoopTable, MemoryEffectTable, MemorySsaTable, MoveTable,
    Mutation, OriginTable, PlaceTable, PostdominatorTable, ScalarEvolutionTable, UseTable,
};

/// Cached analyses for one MIR function.
#[derive(Debug)]
pub struct FunctionCache {
    /// The cached alias table.
    alias: Option<Arc<AliasTable>>,
    /// The cached constants.
    constant: Option<Arc<ConstantTable>>,
    /// The cached control flow.
    control: Option<Arc<ControlTable>>,
    /// The cached value definitions.
    definition: Option<Arc<DefinitionTable>>,
    /// The cached dominators.
    dominator: Option<Arc<DominatorTable>>,
    /// Scalar evolution and its memoized queries.
    evolution: Option<ScalarEvolutionTable>,
    /// The cached move-path initialization.
    initialization: Option<Arc<InitializationTable>>,
    /// The cached liveness table.
    liveness: Option<Arc<LivenessTable>>,
    /// The cached loops.
    loops: Option<Arc<LoopTable>>,
    /// The cached memory effects of individual operations.
    pub(super) memory_effect: Option<Arc<MemoryEffectTable>>,
    /// The cached memory versions.
    pub(super) ssa: Option<Arc<MemorySsaTable>>,
    /// The cached move paths.
    moves: Option<Arc<MoveTable>>,
    /// The cached canonical places.
    place: Option<Arc<PlaceTable>>,
    /// The cached postdominators.
    postdominator: Option<Arc<PostdominatorTable>>,
    /// The cached borrow origin.
    pub(super) origin: Option<Arc<OriginTable>>,
    /// The cached value uses.
    uses: Option<Arc<UseTable>>,
    /// The target layout used by these analyses.
    target_layout: mir::TargetLayout,
}

impl FunctionCache {
    /// Create empty function analyses.
    pub fn new() -> Self {
        Self::with_target_layout(mir::TargetLayout::default())
    }

    /// Create empty function analyses with the given target layout.
    pub fn with_target_layout(target_layout: mir::TargetLayout) -> Self {
        Self {
            alias: None,
            constant: None,
            control: None,
            definition: None,
            dominator: None,
            evolution: None,
            initialization: None,
            liveness: None,
            loops: None,
            memory_effect: None,
            ssa: None,
            moves: None,
            place: None,
            postdominator: None,
            origin: None,
            uses: None,
            target_layout,
        }
    }

    /// Return the target layout for this analysis run.
    pub fn target_layout(&self) -> mir::TargetLayout {
        self.target_layout
    }

    /// Return alias relationships, analysing them when required.
    pub fn alias(&mut self, function: &mir::Function, tree: &mir::Tree) -> Arc<AliasTable> {
        // reuse the result while its inputs remain unchanged
        if let Some(result) = &self.alias {
            return result.clone();
        }

        // analyse the required inputs
        let definitions = self.definition(function, tree);
        let constants = self.constant(function, tree);
        let places = self.place(function, tree);

        // analyse and cache the result
        let result = Arc::new(AliasTable::analyse(
            function,
            &definitions,
            constants,
            &places,
            self.target_layout(),
            tree,
        ));
        self.alias = Some(result.clone());

        result
    }

    /// Return known constants, analysing them when required.
    pub fn constant(&mut self, function: &mir::Function, tree: &mir::Tree) -> Arc<ConstantTable> {
        // reuse the result while its inputs remain unchanged
        if let Some(result) = &self.constant {
            return result.clone();
        }

        // analyse the required inputs
        let control = self.control(function, tree);

        // analyse and cache the result
        let result = Arc::new(ConstantTable::analyse(
            function,
            &control,
            self.target_layout(),
            tree,
        ));
        self.constant = Some(result.clone());

        result
    }

    /// Return the control-flow graph, analysing it when required.
    pub fn control(&mut self, function: &mir::Function, tree: &mir::Tree) -> Arc<ControlTable> {
        // reuse the result while its inputs remain unchanged
        if let Some(result) = &self.control {
            return result.clone();
        }

        // analyse and cache the result
        let result = Arc::new(ControlTable::analyse(function, tree));
        self.control = Some(result.clone());

        result
    }

    /// Return value definitions, analysing them when required.
    pub fn definition(
        &mut self,
        function: &mir::Function,
        tree: &mir::Tree,
    ) -> Arc<DefinitionTable> {
        // reuse the result while its inputs remain unchanged
        if let Some(result) = &self.definition {
            return result.clone();
        }

        // analyse and cache the result
        let result = Arc::new(DefinitionTable::analyse(function, tree));
        self.definition = Some(result.clone());

        result
    }

    /// Return dominators, analysing them when required.
    pub fn dominator(&mut self, function: &mir::Function, tree: &mir::Tree) -> Arc<DominatorTable> {
        // reuse the result while its inputs remain unchanged
        if let Some(result) = &self.dominator {
            return result.clone();
        }

        // analyse the required inputs
        let control = self.control(function, tree);

        // analyse and cache the result
        let result = Arc::new(DominatorTable::analyse(control));
        self.dominator = Some(result.clone());

        result
    }

    /// Return exclusive access to scalar evolution and its memoized queries.
    pub fn evolution(
        &mut self,
        function: &mir::Function,
        tree: &mir::Tree,
    ) -> &mut ScalarEvolutionTable {
        // reuse the query state or construct it from the function's analyses
        let evolution = match self.evolution.take() {
            Some(evolution) => evolution,
            None => {
                let definitions = self.definition(function, tree);
                let dominators = self.dominator(function, tree);
                let loops = self.loops(function, tree);

                ScalarEvolutionTable::analyse(
                    function,
                    definitions,
                    dominators,
                    loops,
                    self.target_layout(),
                    tree,
                )
            }
        };

        self.evolution.insert(evolution)
    }

    /// Return value liveness, analysing it when required.
    pub fn liveness(&mut self, function: &mir::Function, tree: &mir::Tree) -> Arc<LivenessTable> {
        // reuse the result while its inputs remain unchanged
        if let Some(result) = &self.liveness {
            return result.clone();
        }

        // analyse and cache the result
        let result = Arc::new(LivenessTable::analyse(function, tree));
        self.liveness = Some(result.clone());

        result
    }

    /// Return move-path initialization, analysing it when required.
    pub fn initialization(
        &mut self,
        function: &mir::Function,
        tree: &mir::Tree,
    ) -> Arc<InitializationTable> {
        // reuse the result while its inputs remain unchanged
        if let Some(result) = &self.initialization {
            return result.clone();
        }

        // analyse the required inputs
        let control = self.control(function, tree);
        let paths = self.moves(function, tree);
        let places = self.place(function, tree);

        // analyse and cache the result
        let result = Arc::new(InitializationTable::analyse(
            function, &control, paths, places, tree,
        ));
        self.initialization = Some(result.clone());

        result
    }

    /// Return loops, analysing them when required.
    pub fn loops(&mut self, function: &mir::Function, tree: &mir::Tree) -> Arc<LoopTable> {
        // reuse the result while its inputs remain unchanged
        if let Some(result) = &self.loops {
            return result.clone();
        }

        // analyse the required inputs
        let control = self.control(function, tree);
        let dominators = self.dominator(function, tree);

        // analyse and cache the result
        let result = Arc::new(LoopTable::analyse(function, &control, &dominators));
        self.loops = Some(result.clone());

        result
    }

    /// Return the memory effects of individual operations.
    pub fn memory_effect(
        &mut self,
        function: &mir::Function,
        tree: &mir::Tree,
        accesses: &mir::AccessTable,
        effects: &mir::EffectTable,
    ) -> Arc<MemoryEffectTable> {
        // reuse the classification while its inputs remain unchanged
        if let Some(result) = &self.memory_effect {
            return result.clone();
        }

        // classify operations using the current constants and effect tables
        let constants = self.constant(function, tree);
        let control = self.control(function, tree);
        let result = Arc::new(MemoryEffectTable::analyse(
            function,
            &constants,
            &control,
            accesses,
            effects,
            self.target_layout(),
            tree,
        ));
        self.memory_effect = Some(result.clone());

        result
    }

    /// Return memory SSA for the function.
    pub fn ssa(
        &mut self,
        function: &mir::Function,
        tree: &mir::Tree,
        accesses: &mir::AccessTable,
        effects: &mir::EffectTable,
    ) -> Arc<MemorySsaTable> {
        // reuse the graph while its inputs remain unchanged
        if let Some(result) = &self.ssa {
            return result.clone();
        }

        // compute the control graph, dominators, and operation effects
        let control = self.control(function, tree);
        let dominators = self.dominator(function, tree);
        let effects = self.memory_effect(function, tree, accesses, effects);

        // construct and cache memory SSA
        let result = Arc::new(MemorySsaTable::analyse(
            function,
            &control,
            &dominators,
            &effects,
            tree,
        ));
        self.ssa = Some(result.clone());

        result
    }

    /// Return move paths, analysing them when required.
    pub fn moves(&mut self, function: &mir::Function, tree: &mir::Tree) -> Arc<MoveTable> {
        // reuse the result while its inputs remain unchanged
        if let Some(result) = &self.moves {
            return result.clone();
        }

        // analyse the required inputs
        let places = self.place(function, tree);

        // analyse and cache the result
        let result = Arc::new(MoveTable::analyse(function, &places, tree));
        self.moves = Some(result.clone());

        result
    }

    /// Return canonical places, analysing them when required.
    pub fn place(&mut self, function: &mir::Function, tree: &mir::Tree) -> Arc<PlaceTable> {
        // reuse the result while its inputs remain unchanged
        if let Some(result) = &self.place {
            return result.clone();
        }

        // reuse the function control graph
        let control = self.control(function, tree);

        // analyse and cache the result
        let result = Arc::new(PlaceTable::analyse(function, &control, tree));
        self.place = Some(result.clone());

        result
    }

    /// Return borrow origins, analysing them when required.
    pub fn origin(&mut self, function: &mir::Function, tree: &mir::Tree) -> Arc<OriginTable> {
        // reuse the result while its inputs remain unchanged
        if let Some(result) = &self.origin {
            return result.clone();
        }

        // analyse the required inputs
        let control = self.control(function, tree);
        let places = self.place(function, tree);

        // analyse and cache the result
        let result = Arc::new(OriginTable::analyse(function, &control, &places, tree));
        self.origin = Some(result.clone());

        result
    }

    /// Return postdominators, analysing them when required.
    pub fn postdominator(
        &mut self,
        function: &mir::Function,
        tree: &mir::Tree,
    ) -> Arc<PostdominatorTable> {
        // reuse the result while its inputs remain unchanged
        if let Some(result) = &self.postdominator {
            return result.clone();
        }

        // analyse the required inputs
        let control = self.control(function, tree);

        // analyse and cache the result
        let result = Arc::new(PostdominatorTable::analyse(control));
        self.postdominator = Some(result.clone());

        result
    }

    /// Return value uses, analysing them when required.
    pub fn uses(&mut self, function: &mir::Function, tree: &mir::Tree) -> Arc<UseTable> {
        // reuse the result while its inputs remain unchanged
        if let Some(result) = &self.uses {
            return result.clone();
        }

        // analyse and cache the result
        let result = Arc::new(UseTable::analyse(function, tree));
        self.uses = Some(result.clone());

        result
    }

    /// Discard module-dependent results before running passes restricted to this function cache.
    pub fn invalidate_module_dependencies(&mut self) {
        self.memory_effect = None;
        self.ssa = None;
        self.origin = None;
    }

    /// Drop every analysis the given mutation invalidates.
    pub fn invalidate(&mut self, mutation: Mutation) {
        // invalidate aliases
        if AliasTable::INVALIDATED_BY.intersects(mutation) {
            self.alias = None;
        }

        // invalidate constants
        if ConstantTable::INVALIDATED_BY.intersects(mutation) {
            self.constant = None;
        }

        // invalidate control flow
        if ControlTable::INVALIDATED_BY.intersects(mutation) {
            self.control = None;
        }

        // invalidate definitions
        if DefinitionTable::INVALIDATED_BY.intersects(mutation) {
            self.definition = None;
        }

        // invalidate dominators
        if DominatorTable::INVALIDATED_BY.intersects(mutation) {
            self.dominator = None;
        }

        // invalidate scalar evolution
        if ScalarEvolutionTable::INVALIDATED_BY.intersects(mutation) {
            self.evolution = None;
        }

        // invalidate liveness
        if LivenessTable::INVALIDATED_BY.intersects(mutation) {
            self.liveness = None;
        }

        // invalidate initialization
        if InitializationTable::INVALIDATED_BY.intersects(mutation) {
            self.initialization = None;
        }

        // invalidate loops
        if LoopTable::INVALIDATED_BY.intersects(mutation) {
            self.loops = None;
        }

        // invalidate operation memory effects
        if MemoryEffectTable::INVALIDATED_BY.intersects(mutation) {
            self.memory_effect = None;
        }

        // invalidate memory versions
        if MemorySsaTable::INVALIDATED_BY.intersects(mutation) {
            self.ssa = None;
        }

        // invalidate move paths
        if MoveTable::INVALIDATED_BY.intersects(mutation) {
            self.moves = None;
        }

        // invalidate places
        if PlaceTable::INVALIDATED_BY.intersects(mutation) {
            self.place = None;
        }

        // invalidate postdominators
        if PostdominatorTable::INVALIDATED_BY.intersects(mutation) {
            self.postdominator = None;
        }

        // invalidate borrow origins
        if OriginTable::INVALIDATED_BY.intersects(mutation) {
            self.origin = None;
        }

        // invalidate uses
        if UseTable::INVALIDATED_BY.intersects(mutation) {
            self.uses = None;
        }
    }
}

impl Default for FunctionCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestModule;
    use crate::{MemoryAccessEffect, MemoryRegion};

    /// Classify accesses independently of memory SSA and refresh changed access metadata.
    #[test]
    fn test_classify_and_invalidate_accesses() {
        let mut program = TestModule::new(
            r#"
function test(): int32 {
    local l0: int32

entry:
    v0: int32 = 1
    local.set l0, v0
    v1: int32 = local.get l0
    return v1
}
"#,
        );
        let function_id = program.entry_function_id();
        let function = program.tree.get(function_id);
        let local = function.locals()[0];
        let block = function.block(0);
        let instructions = program.tree.get(block).instructions.clone();
        let mut analyses = FunctionCache::new();
        let effects =
            analyses.memory_effect(function, &program.tree, &program.accesses, &program.effects);

        // check every instruction and the terminator without constructing memory SSA
        let mut expected = vec![
            vec![],
            vec![MemoryAccessEffect {
                reads: false,
                writes: true,
                is_volatile: false,
                is_barrier: false,
                region: MemoryRegion::Local(local),
            }],
            vec![MemoryAccessEffect {
                reads: true,
                writes: false,
                is_volatile: false,
                is_barrier: false,
                region: MemoryRegion::Local(local),
            }],
        ];
        let actual = instructions
            .iter()
            .map(|instruction| {
                effects
                    .instruction_effects(*instruction)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
        assert_eq!(
            effects.terminator_effects(block).collect::<Vec<_>>(),
            Vec::<&MemoryAccessEffect>::new()
        );
        assert!(analyses.ssa.is_none());
        assert!(analyses.dominator.is_none());

        // change the read to volatile through explicit access metadata
        program.insert_accesses(
            instructions[2],
            vec![mir::MemoryAccess {
                operation: mir::MemoryOperation::Read,
                target: mir::MemoryTarget::Local(local),
                byte_len: Some(4),
                alignment_bytes: None,
                order: mir::MemoryAccessOrder::Volatile,
            }],
        );
        analyses.invalidate(Mutation::MEMORY);
        let function = program.tree.get(function_id);
        let updated =
            analyses.memory_effect(function, &program.tree, &program.accesses, &program.effects);

        // preserve the other accesses while observing the changed ordering
        expected[2][0].is_volatile = true;
        let actual = instructions
            .iter()
            .map(|instruction| {
                updated
                    .instruction_effects(*instruction)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        assert_eq!(actual, expected);
        assert_eq!(
            updated.terminator_effects(block).collect::<Vec<_>>(),
            Vec::<&MemoryAccessEffect>::new()
        );
        assert!(!Arc::ptr_eq(&effects, &updated));
    }

    /// Reuse graph analyses for value changes and rebuild them after control changes.
    #[test]
    fn test_invalidate_control_and_dominators() {
        let (mut tree, function_id) = TestModule::parse_function(
            r#"
function test(v0: boolean): void {
entry(v0: boolean):
    branch v0 => middle | done

middle:
    jump done

done:
    return
}
"#,
        );
        let function = tree.get(function_id).clone();
        let entry = function.block(0);
        let middle = function.block(1);
        let exit = function.block(2);
        let mut cache = FunctionCache::new();
        let control = cache.control(&function, &tree);
        let dominators = cache.dominator(&function, &tree);
        let postdominators = cache.postdominator(&function, &tree);
        assert_eq!(postdominators.immediate_postdominator(entry), Some(exit));
        assert_eq!(dominators.immediate_dominator(exit), Some(entry));

        // preserve cached graph results when only values change
        cache.invalidate(Mutation::VALUE);
        assert!(Arc::ptr_eq(&control, &cache.control(&function, &tree)));
        assert!(Arc::ptr_eq(&dominators, &cache.dominator(&function, &tree)));
        assert!(Arc::ptr_eq(
            &postdominators,
            &cache.postdominator(&function, &tree)
        ));

        // remove the direct edge from entry to exit and invalidate its dependents
        let arguments = tree.add_values(&[]);
        let terminator = tree.get(entry).terminator;
        *tree.get_mut(terminator) = mir::Terminator::Jump {
            target: mir::BlockTarget::new(middle, arguments),
        };
        cache.invalidate(Mutation::CONTROL);
        let changed_control = cache.control(&function, &tree);
        let changed_dominators = cache.dominator(&function, &tree);
        let changed_postdominators = cache.postdominator(&function, &tree);
        assert!(!Arc::ptr_eq(&control, &changed_control));
        assert!(!Arc::ptr_eq(&dominators, &changed_dominators));
        assert!(!Arc::ptr_eq(&postdominators, &changed_postdominators));
        assert_eq!(
            changed_postdominators.immediate_postdominator(entry),
            Some(middle)
        );
        assert_eq!(
            changed_control.predecessors(exit).collect::<Vec<_>>(),
            vec![middle]
        );
        assert_eq!(changed_dominators.immediate_dominator(exit), Some(middle));
    }
}
