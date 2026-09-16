use std::sync::Arc;

use crate as mir;
use crate::{
    AliasTable, Analysis, CallTable, ConstantTable, ControlTable, DefinitionTable, DominatorTable,
    EffectTable, EscapeTable, FunctionCache, GlobalAccessTable, InitializationTable, LinkTable,
    LivenessTable, LoopTable, MemoryEffectTable, MemorySsaTable, MoveTable, Mutation, OriginTable,
    PlaceTable, PostdominatorTable, ResolutionTable, ScalarEvolutionTable, UseTable,
};

/// Cached analyses for one MIR module.
#[derive(Debug)]
pub struct ModuleCache {
    /// The cached call table.
    call: Option<Arc<CallTable>>,
    /// The cached dispatch resolutions.
    resolution: Option<Arc<ResolutionTable>>,
    /// The cached function effects.
    effect: Option<Arc<EffectTable>>,
    /// The cached allocation escape results.
    escape: Option<Arc<mir::NodeTable<mir::Function, Arc<EscapeTable>>>>,
    /// The cached global accesses.
    globals: Option<Arc<GlobalAccessTable>>,
    /// The cached symbol links.
    link: Option<Arc<LinkTable>>,
    /// Function caches sorted by function id.
    functions: Vec<(mir::FunctionId, FunctionCache)>,
    /// The target layout used by these analyses.
    target_layout: mir::TargetLayout,
}

macro_rules! function_analysis_through_module {
    (
        $method:ident,
        $analysis:ty,
        $tree:ty,
        $description:literal
        $(, $parameter:ident: $parameter_type:ty)*
    ) => {
        #[doc = $description]
        pub fn $method(
            &mut self,
            function_id: mir::FunctionId,
            tree: $tree,
            $($parameter: $parameter_type,)*
        ) -> Arc<$analysis> {
            let analyses = self.function(function_id);

            analyses.$method(function_id, tree, $($parameter,)*)
        }
    };
}

impl ModuleCache {
    /// Create empty module analyses.
    pub fn new() -> Self {
        Self::with_target_layout(mir::TargetLayout::default())
    }

    /// Create empty module analyses with the given target layout.
    pub fn with_target_layout(target_layout: mir::TargetLayout) -> Self {
        Self {
            call: None,
            resolution: None,
            effect: None,
            escape: None,
            globals: None,
            link: None,
            functions: Vec::new(),
            target_layout,
        }
    }

    /// Return the target layout used by these analyses.
    pub fn target_layout(&self) -> mir::TargetLayout {
        self.target_layout
    }

    /// Return the call graph, analysing it when required.
    pub fn call(&mut self, tree: &mir::Tree, dispatch: &mir::DispatchTable) -> Arc<CallTable> {
        // reuse the result while its inputs remain unchanged
        if let Some(result) = &self.call {
            return result.clone();
        }

        // analyse the required inputs
        let resolution = self.resolution(tree, dispatch);

        // analyse and cache the result
        let result = Arc::new(CallTable::analyse(&resolution, tree));
        self.call = Some(result.clone());

        result
    }

    /// Return resolved callees, analysing them when required.
    pub fn resolution(
        &mut self,
        tree: &mir::Tree,
        dispatch: &mir::DispatchTable,
    ) -> Arc<ResolutionTable> {
        // reuse the result while its inputs remain unchanged
        if let Some(result) = &self.resolution {
            return result.clone();
        }

        // analyse and cache the result
        let result = Arc::new(ResolutionTable::analyse(dispatch, None, tree));
        self.resolution = Some(result.clone());

        result
    }

    /// Return function effects, analysing them when required.
    pub fn effect(
        &mut self,
        tree: &mir::Tree,
        effects: &mir::EffectTable,
        dispatch: &mir::DispatchTable,
    ) -> Arc<EffectTable> {
        // reuse the result while its inputs remain unchanged
        if let Some(result) = &self.effect {
            return result.clone();
        }

        // analyse the required inputs
        let resolution = self.resolution(tree, dispatch);
        let calls = self.call(tree, dispatch);

        // analyse and cache the result
        let result = Arc::new(EffectTable::analyse(&resolution, &calls, effects, tree));
        self.effect = Some(result.clone());

        result
    }

    /// Return symbol references, analysing them when required.
    pub fn link(
        &mut self,
        tree: &mir::Tree,
        effects: &mir::EffectTable,
        dispatch: &mir::DispatchTable,
        drops: &mir::DropTable,
    ) -> Arc<LinkTable> {
        // reuse the result while its inputs remain unchanged
        if let Some(result) = &self.link {
            return result.clone();
        }

        // analyse the required inputs
        let calls = self.call(tree, dispatch);

        // analyse and cache the result
        let result = Arc::new(LinkTable::analyse(&calls, effects, drops, tree));
        self.link = Some(result.clone());

        result
    }

    /// Return global accesses, analysing them when required.
    pub fn globals(
        &mut self,
        tree: &mir::Tree,
        effects: &mir::EffectTable,
        dispatch: &mir::DispatchTable,
    ) -> Arc<GlobalAccessTable> {
        // reuse the result while its inputs remain unchanged
        if let Some(result) = &self.globals {
            return result.clone();
        }

        // compute the call graph and function effects
        let calls = self.call(tree, dispatch);
        let effects = self.effect(tree, effects, dispatch);

        // analyse and cache the global accesses
        let result = Arc::new(GlobalAccessTable::analyse(&calls, &effects, tree));
        self.globals = Some(result.clone());

        result
    }

    /// Return exclusive access to one function's scalar evolution.
    pub fn evolution(
        &mut self,
        function: mir::FunctionId,
        tree: &mir::Tree,
    ) -> &mut ScalarEvolutionTable {
        self.function(function).evolution(function, tree)
    }

    /// Return physical alias analysis for one function and its canonical layouts.
    pub fn alias(
        &mut self,
        function: mir::FunctionId,
        layouts: Arc<mir::LayoutTable>,
        tree: &mir::Tree,
    ) -> Result<Arc<AliasTable>, mir::LayoutError> {
        self.function(function).alias(function, layouts, tree)
    }

    function_analysis_through_module!(
        constant,
        ConstantTable,
        &mir::Tree,
        "Return function constant propagation."
    );
    function_analysis_through_module!(
        control,
        ControlTable,
        &mir::Tree,
        "Return the function control-flow graph."
    );
    function_analysis_through_module!(
        definition,
        DefinitionTable,
        &mir::Tree,
        "Return function value definitions."
    );
    function_analysis_through_module!(
        dominator,
        DominatorTable,
        &mir::Tree,
        "Return the function dominator tree."
    );

    /// Return allocation escape results for one function.
    pub fn escape(
        &mut self,
        function: mir::FunctionId,
        tree: &mir::Tree,
        effects: &mir::EffectTable,
        dispatch: &mir::DispatchTable,
    ) -> Arc<EscapeTable> {
        // reuse results while the function bodies remain unchanged
        if let Some(result) = &self.escape {
            return result.get(function).clone();
        }

        // solve the module's recursive calls before selecting the function result
        let resolution = self.resolution(tree, dispatch);
        let calls = self.call(tree, dispatch);
        let results = Arc::new(mir::EscapeBody::analyse_module(
            &resolution,
            &calls,
            effects,
            tree,
        ));
        let result = results.get(function).clone();
        self.escape = Some(results);

        result
    }

    function_analysis_through_module!(
        liveness,
        LivenessTable,
        &mir::Tree,
        "Return function value liveness."
    );
    function_analysis_through_module!(
        loops,
        LoopTable,
        &mir::Tree,
        "Return function loop analysis."
    );
    function_analysis_through_module!(
        ssa,
        MemorySsaTable, &mir::Tree,
        "Return function memory versions.",
        effects: &mir::EffectTable
    );
    function_analysis_through_module!(
        memory_effect, MemoryEffectTable, &mir::Tree, "Return function memory effects.",
        effects: &mir::EffectTable
    );
    function_analysis_through_module!(moves, MoveTable, &mir::Tree, "Return function move paths.");
    function_analysis_through_module!(
        place,
        PlaceTable,
        &mir::Tree,
        "Return function canonical places."
    );
    function_analysis_through_module!(
        origin,
        OriginTable,
        &mir::Tree,
        "Return function borrow origin."
    );
    function_analysis_through_module!(
        postdominator,
        PostdominatorTable,
        &mir::Tree,
        "Return function postdominators."
    );
    function_analysis_through_module!(
        initialization,
        InitializationTable,
        &mir::Tree,
        "Return function move-path initialization."
    );
    function_analysis_through_module!(uses, UseTable, &mir::Tree, "Return function value uses.");

    /// Get or create the analysis cache for one function.
    pub fn function(&mut self, function_id: mir::FunctionId) -> &mut FunctionCache {
        // maintain function caches in id order
        let index = self
            .functions
            .binary_search_by_key(&function_id, |(function, _)| *function);
        let index = match index {
            Ok(index) => index,
            Err(index) => {
                let analyses = FunctionCache::with_target_layout(self.target_layout);
                self.functions.insert(index, (function_id, analyses));

                index
            }
        };

        &mut self.functions[index].1
    }

    /// Invalidate one changed body and all analyses affected through module dependencies.
    pub fn invalidate_function(&mut self, function_id: mir::FunctionId, mutation: Mutation) {
        // invalidate module analyses and their function dependencies
        self.invalidate_module(mutation);
        let shared = mutation.intersection(
            Mutation::MEMORY
                .union(Mutation::EFFECT)
                .union(Mutation::LAYOUT)
                .union(Mutation::SYMBOL)
                .union(Mutation::DISPATCH)
                .union(Mutation::DROP),
        );

        // apply body changes to the selected function and shared input changes to the others
        for (cached_function, analyses) in &mut self.functions {
            // invalidate every changed input in the selected function
            if *cached_function == function_id {
                analyses.invalidate(mutation);
            }
            // invalidate shared inputs in the remaining functions
            else {
                analyses.invalidate(shared);
            }
        }
    }

    /// Invalidate analyses after a rewrite that may affect any function in the module.
    pub fn invalidate(&mut self, mutation: Mutation) {
        // invalidate module analyses and their function dependencies
        self.invalidate_module(mutation);

        // invalidate changed inputs in every function cache
        for (_, analyses) in &mut self.functions {
            analyses.invalidate(mutation);
        }
    }

    /// Invalidate module results and their consumers after invalidating each changed function.
    pub fn invalidate_module(&mut self, mutation: Mutation) {
        // preserve every cached result when the module is unchanged
        if mutation.is_none() {
            return;
        }

        // determine which shared results need recomputation
        let resolution_changed = ResolutionTable::INVALIDATED_BY.intersects(mutation);
        let effects_changed = EffectTable::INVALIDATED_BY.intersects(mutation);

        // invalidate the module's call graph
        if CallTable::INVALIDATED_BY.intersects(mutation) {
            self.call = None;
        }

        // invalidate dispatch resolution
        if resolution_changed {
            self.resolution = None;
        }

        // invalidate function effects
        if effects_changed {
            self.effect = None;
        }

        // invalidate allocation escapes after body changes
        if EscapeTable::INVALIDATED_BY.intersects(mutation) {
            self.escape = None;
        }

        // invalidate global accesses after changes to their inputs
        if GlobalAccessTable::INVALIDATED_BY.intersects(mutation) {
            self.globals = None;
        }

        // invalidate symbol links and their copied effect results
        if LinkTable::INVALIDATED_BY.intersects(mutation) || effects_changed {
            self.link = None;
        }

        // invalidate function analyses that consume changed module results
        for (_, analyses) in &mut self.functions {
            // invalidate memory versions after function effects change
            if effects_changed {
                analyses.memory_effect = None;
                analyses.ssa = None;
            }
        }
    }
}

impl Default for ModuleCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestModule;

    /// Preserve unrelated body analyses while rebuilding shared effects and memory dependencies.
    #[test]
    fn test_invalidate_one_function() {
        let mut test = TestModule::new(
            r#"
function first(): int32 {
entry:
    v0: int32 = 1
    return v0
}

function second(): int32 {
entry:
    v0: int32 = 2
    return v0
}
"#,
        );
        let functions: Vec<_> = test
            .tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .collect();

        // compute the analyses whose reuse is checked below
        let mut cache = ModuleCache::new();
        let first_control = cache.control(functions[0], &test.tree);
        let second_control = cache.control(functions[1], &test.tree);
        let first_constant = cache.constant(functions[0], &test.tree);
        let second_constant = cache.constant(functions[1], &test.tree);
        let effects = cache.effect(&test.tree, &test.effects, &test.dispatch);
        let second_memory = cache.ssa(functions[1], &test.tree, &effects);

        // replace the first function's constant before invalidating its cached answer
        let block = test.tree.get(functions[0]).entry().expect("function entry");
        let instruction = test.tree.get(block).instructions[0];
        let mir::Instruction::Const { destination, value } = test.tree.get_mut(instruction) else {
            panic!("expected constant instruction");
        };
        let destination = *destination;
        assert_eq!(
            first_constant.constant(destination),
            Some(&mir::Constant::int32(1))
        );
        *value = mir::Constant::int32(3);

        // preserve both control graphs and the unchanged function's constants
        cache.invalidate_function(functions[0], Mutation::VALUE);
        assert!(Arc::ptr_eq(
            &first_control,
            &cache.control(functions[0], &test.tree)
        ));
        assert!(Arc::ptr_eq(
            &second_control,
            &cache.control(functions[1], &test.tree)
        ));
        assert!(!Arc::ptr_eq(
            &first_constant,
            &cache.constant(functions[0], &test.tree)
        ));
        assert!(Arc::ptr_eq(
            &second_constant,
            &cache.constant(functions[1], &test.tree)
        ));

        // require the rebuilt analysis to observe the rewritten constant
        let constants = cache.constant(functions[0], &test.tree);
        assert_eq!(
            constants.constant(destination),
            Some(&mir::Constant::int32(3))
        );

        // invalidate shared effects and dependent memory analyses after an operand change
        let new_effects = cache.effect(&test.tree, &test.effects, &test.dispatch);
        assert!(!Arc::ptr_eq(&effects, &new_effects));
        assert!(!Arc::ptr_eq(
            &second_memory,
            &cache.ssa(functions[1], &test.tree, &new_effects)
        ));

        // preserve the unchanged function's control graph after a branch change
        cache.invalidate_function(functions[0], Mutation::CONTROL);
        assert!(!Arc::ptr_eq(
            &first_control,
            &cache.control(functions[0], &test.tree)
        ));
        assert!(Arc::ptr_eq(
            &second_control,
            &cache.control(functions[1], &test.tree)
        ));

        // invalidate memory analyses after a layout change
        let memory = cache.ssa(functions[1], &test.tree, &new_effects);
        cache.invalidate_function(functions[0], Mutation::LAYOUT);
        assert!(!Arc::ptr_eq(
            &memory,
            &cache.ssa(functions[1], &test.tree, &new_effects),
        ));
        assert!(Arc::ptr_eq(
            &second_control,
            &cache.control(functions[1], &test.tree)
        ));
    }

    /// Invalidate dispatch-dependent module and function analyses together.
    #[test]
    fn test_invalidate_dispatch_dependencies() {
        let test = TestModule::new(
            r#"
function test(): int32 {
entry:
    v0: int32 = 1
    return v0
}
"#,
        );
        let function = test.entry_function_id();

        // compute the analyses whose reuse is checked below
        let mut cache = ModuleCache::new();
        let control = cache.control(function, &test.tree);
        let resolution = cache.resolution(&test.tree, &test.dispatch);
        let origin = cache.origin(function, &test.tree);
        let calls = cache.call(&test.tree, &test.dispatch);
        let effects = cache.effect(&test.tree, &test.effects, &test.dispatch);

        // invalidate the dispatch table and its dependent analyses
        cache.invalidate(Mutation::DISPATCH);
        let new_resolution = cache.resolution(&test.tree, &test.dispatch);
        assert!(!Arc::ptr_eq(&resolution, &new_resolution));
        assert!(Arc::ptr_eq(&origin, &cache.origin(function, &test.tree)));
        assert!(!Arc::ptr_eq(
            &calls,
            &cache.call(&test.tree, &test.dispatch)
        ));
        assert!(!Arc::ptr_eq(
            &effects,
            &cache.effect(&test.tree, &test.effects, &test.dispatch)
        ));

        // check that dispatch changes preserve the control graph
        assert!(Arc::ptr_eq(&control, &cache.control(function, &test.tree)));
    }
}
