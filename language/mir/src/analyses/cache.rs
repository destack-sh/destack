use std::sync::Arc;

use crate as mir;
use crate::EffectTable;

use super::{
    AliasTable, CallTable, ConstantTable, ControlTable, DefinitionTable, DominatorTable,
    EscapeTable, InitializationTable, LinkTable, LivenessTable, LoopTable, MemoryTable, MoveTable,
    Mutation, OriginTable, PlaceTable, PostdominatorTable, ResolutionTable, UseTable,
};

/// Cached analyses for one MIR module.
#[derive(Debug)]
pub struct AnalysisCache {
    /// The cached call table.
    call: Option<Arc<CallTable>>,
    /// The cached dispatch resolutions.
    resolution: Option<Arc<ResolutionTable>>,
    /// The cached function effects.
    effect: Option<Arc<EffectTable>>,
    /// The cached symbol links.
    link: Option<Arc<LinkTable>>,
    /// Function caches sorted by function id.
    functions: Vec<(mir::FunctionId, FunctionCache)>,
    /// The analysis options.
    options: Arc<AnalysisOptions>,
}

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
    /// The cached escapes.
    escape: Option<Arc<EscapeTable>>,
    /// The cached move-path initialization.
    initialization: Option<Arc<InitializationTable>>,
    /// The cached liveness table.
    liveness: Option<Arc<LivenessTable>>,
    /// The cached loops.
    loops: Option<Arc<LoopTable>>,
    /// The cached memory versions.
    memory: Option<Arc<MemoryTable>>,
    /// The cached move paths.
    moves: Option<Arc<MoveTable>>,
    /// The cached canonical places.
    place: Option<Arc<PlaceTable>>,
    /// The cached postdominators.
    postdominator: Option<Arc<PostdominatorTable>>,
    /// The cached borrow origin.
    origin: Option<Arc<OriginTable>>,
    /// The cached value uses.
    uses: Option<Arc<UseTable>>,
    /// The analysis options.
    options: Arc<AnalysisOptions>,
}

/// Inputs shared by MIR analyses.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AnalysisOptions {
    /// Target layout used by layout sensitive analyses.
    pub target_layout: mir::TargetLayout,
}

impl AnalysisOptions {
    /// Create analysis options for one target.
    pub fn new(target_layout: mir::TargetLayout) -> Self {
        Self { target_layout }
    }
}

/// Base trait for all analyses.
pub(crate) trait Analysis: 'static + Send + Sync + Sized {
    /// The mutations that invalidate this analysis.
    const INVALIDATED_BY: Mutation = Mutation::ALL;
}

macro_rules! function_analysis {
    (
        $method:ident,
        $field:ident,
        $analysis:ty,
        $description:literal
        $(, $parameter:ident: $parameter_type:ty)*
    ) => {
        #[doc = $description]
        pub fn $method(
            &mut self,
            function: &mir::Function,
            tree: &mir::Tree,
            $($parameter: $parameter_type,)*
        ) -> Arc<$analysis> {
            if let Some(result) = &self.$field {
                return result.clone();
            }

            let result = Arc::new(<$analysis>::compute(
                function,
                tree,
                self,
                $($parameter,)*
            ));
            self.$field = Some(result.clone());

            result
        }
    };
}

macro_rules! module_analysis {
    (
        $method:ident,
        $field:ident,
        $analysis:ty,
        $description:literal
        $(, $parameter:ident: $parameter_type:ty)*
    ) => {
        #[doc = $description]
        pub fn $method(
            &mut self,
            tree: &mir::Tree,
            $($parameter: $parameter_type,)*
        ) -> Arc<$analysis> {
            if let Some(result) = &self.$field {
                return result.clone();
            }

            let result = Arc::new(<$analysis>::compute(tree, self, $($parameter,)*));
            self.$field = Some(result.clone());

            result
        }
    };
}

macro_rules! function_analysis_through_module {
    (
        $method:ident,
        $analysis:ty,
        $description:literal
        $(, $parameter:ident: $parameter_type:ty)*
    ) => {
        #[doc = $description]
        pub fn $method(
            &mut self,
            function_id: mir::FunctionId,
            tree: &mir::Tree,
            $($parameter: $parameter_type,)*
        ) -> Arc<$analysis> {
            let analyses = self.function(function_id);
            let function = tree.get(function_id);

            analyses.$method(function, tree, $($parameter,)*)
        }
    };
}

impl FunctionCache {
    /// Create empty function analyses.
    pub fn new() -> Self {
        Self::with_options(AnalysisOptions::default())
    }

    /// Create empty function analyses with the given options.
    pub fn with_options(options: AnalysisOptions) -> Self {
        Self::from_options(Arc::new(options))
    }

    /// Create empty function analyses sharing existing options.
    fn from_options(options: Arc<AnalysisOptions>) -> Self {
        Self {
            alias: None,
            constant: None,
            control: None,
            definition: None,
            dominator: None,
            escape: None,
            initialization: None,
            liveness: None,
            loops: None,
            memory: None,
            moves: None,
            place: None,
            postdominator: None,
            origin: None,
            uses: None,
            options,
        }
    }

    /// Return the analysis options.
    pub fn options(&self) -> &AnalysisOptions {
        &self.options
    }

    /// Return the target layout for this analysis run.
    pub fn target_layout(&self) -> mir::TargetLayout {
        self.options.target_layout
    }

    function_analysis!(alias, alias, AliasTable, "Return alias analysis.");
    function_analysis!(
        constant,
        constant,
        ConstantTable,
        "Return constant propagation."
    );
    function_analysis!(
        control,
        control,
        ControlTable,
        "Return the control-flow graph."
    );
    function_analysis!(
        definition,
        definition,
        DefinitionTable,
        "Return value definitions."
    );
    function_analysis!(
        dominator,
        dominator,
        DominatorTable,
        "Return the dominator tree."
    );
    function_analysis!(escape, escape, EscapeTable, "Return escape analysis.");
    function_analysis!(liveness, liveness, LivenessTable, "Return value liveness.");
    function_analysis!(
        initialization,
        initialization,
        InitializationTable,
        "Return move-path initialization."
    );
    function_analysis!(loops, loops, LoopTable, "Return loop analysis.");
    function_analysis!(
        memory,
        memory,
        MemoryTable,
        "Return memory versions.",
        accesses: &mir::AccessTable,
        effects: &mir::EffectTable
    );
    function_analysis!(moves, moves, MoveTable, "Return move paths.");
    function_analysis!(place, place, PlaceTable, "Return canonical places.");
    function_analysis!(
        origin,
        origin,
        OriginTable,
        "Return borrow origin.",
        resolution: &mir::ResolutionTable
    );
    function_analysis!(
        postdominator,
        postdominator,
        PostdominatorTable,
        "Return postdominators."
    );
    function_analysis!(uses, uses, UseTable, "Return value uses.");

    /// Discard results that accept body-dependent module analyses as inputs.
    ///
    /// Call this before running function passes with access restricted to this cache.
    pub fn invalidate_module_dependencies(&mut self) {
        self.memory = None;
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

        // invalidate escapes
        if EscapeTable::INVALIDATED_BY.intersects(mutation) {
            self.escape = None;
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

        // invalidate memory versions
        if MemoryTable::INVALIDATED_BY.intersects(mutation) {
            self.memory = None;
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

impl AnalysisCache {
    /// Create empty module analyses.
    pub fn new() -> Self {
        Self::with_options(AnalysisOptions::default())
    }

    /// Create empty module analyses with the given options.
    pub fn with_options(options: AnalysisOptions) -> Self {
        Self {
            call: None,
            resolution: None,
            effect: None,
            link: None,
            functions: Vec::new(),
            options: Arc::new(options),
        }
    }

    /// Return the analysis options.
    pub fn options(&self) -> &AnalysisOptions {
        &self.options
    }

    module_analysis!(
        call,
        call,
        CallTable,
        "Return the call graph.",
        dispatch: &mir::DispatchTable
    );
    module_analysis!(
        resolution,
        resolution,
        ResolutionTable,
        "Return resolution analysis.",
        dispatch_table: &mir::DispatchTable
    );
    module_analysis!(
        effect,
        effect,
        EffectTable,
        "Return function effects.",
        accesses: &mir::AccessTable,
        effects: &mir::EffectTable,
        dispatch: &mir::DispatchTable
    );
    module_analysis!(
        link,
        link,
        LinkTable,
        "Return the link graph.",
        effects: &mir::EffectTable,
        dispatch: &mir::DispatchTable,
        drops: &mir::DropTable
    );

    function_analysis_through_module!(alias, AliasTable, "Return function alias analysis.");
    function_analysis_through_module!(
        constant,
        ConstantTable,
        "Return function constant propagation."
    );
    function_analysis_through_module!(
        control,
        ControlTable,
        "Return the function control-flow graph."
    );
    function_analysis_through_module!(
        definition,
        DefinitionTable,
        "Return function value definitions."
    );
    function_analysis_through_module!(
        dominator,
        DominatorTable,
        "Return the function dominator tree."
    );
    function_analysis_through_module!(escape, EscapeTable, "Return function escape analysis.");
    function_analysis_through_module!(liveness, LivenessTable, "Return function value liveness.");
    function_analysis_through_module!(loops, LoopTable, "Return function loop analysis.");
    function_analysis_through_module!(
        memory,
        MemoryTable,
        "Return function memory versions.",
        accesses: &mir::AccessTable,
        effects: &mir::EffectTable
    );
    function_analysis_through_module!(moves, MoveTable, "Return function move paths.");
    function_analysis_through_module!(place, PlaceTable, "Return function canonical places.");
    function_analysis_through_module!(
        origin,
        OriginTable,
        "Return function borrow origin.",
        resolution: &mir::ResolutionTable
    );
    function_analysis_through_module!(
        postdominator,
        PostdominatorTable,
        "Return function postdominators."
    );
    function_analysis_through_module!(
        initialization,
        InitializationTable,
        "Return function move-path initialization."
    );
    function_analysis_through_module!(uses, UseTable, "Return function value uses.");

    /// Get or create the analysis cache for one function.
    pub fn function(&mut self, function_id: mir::FunctionId) -> &mut FunctionCache {
        // maintain function caches in id order
        let index = self
            .functions
            .binary_search_by_key(&function_id, |(function, _)| *function);
        let index = match index {
            Ok(index) => index,
            Err(index) => {
                let analyses = FunctionCache::from_options(self.options.clone());
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

    /// Invalidate module analyses and the function analyses that consume them.
    ///
    /// Invalidate each changed body separately before completing a group of function passes.
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

        // invalidate symbol links and their copied effect results
        if LinkTable::INVALIDATED_BY.intersects(mutation) || effects_changed {
            self.link = None;
        }

        // invalidate function analyses that consume changed module results
        for (_, analyses) in &mut self.functions {
            // invalidate borrow origins after dispatch resolution changes
            if resolution_changed {
                analyses.origin = None;
            }

            // invalidate memory versions after function effects change
            if effects_changed {
                analyses.memory = None;
            }
        }
    }
}

impl Default for AnalysisCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyses::tests::TestProgram;

    /// Preserve unrelated body analyses while rebuilding shared effects and memory dependencies.
    #[test]
    fn test_invalidate_one_function() {
        let mut test = TestProgram::new(
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
        let mut cache = AnalysisCache::new();
        let first_control = cache.control(functions[0], &test.tree);
        let second_control = cache.control(functions[1], &test.tree);
        let first_constant = cache.constant(functions[0], &test.tree);
        let second_constant = cache.constant(functions[1], &test.tree);
        let effects = cache.effect(&test.tree, &test.accesses, &test.effects, &test.dispatch);
        let second_memory = cache.memory(functions[1], &test.tree, &test.accesses, &effects);

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
        let new_effects = cache.effect(&test.tree, &test.accesses, &test.effects, &test.dispatch);
        assert!(!Arc::ptr_eq(&effects, &new_effects));
        assert!(!Arc::ptr_eq(
            &second_memory,
            &cache.memory(functions[1], &test.tree, &test.accesses, &new_effects)
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
        let memory = cache.memory(functions[1], &test.tree, &test.accesses, &new_effects);
        cache.invalidate_function(functions[0], Mutation::LAYOUT);
        assert!(!Arc::ptr_eq(
            &memory,
            &cache.memory(functions[1], &test.tree, &test.accesses, &new_effects),
        ));
        assert!(Arc::ptr_eq(
            &second_control,
            &cache.control(functions[1], &test.tree)
        ));
    }

    /// Invalidate dispatch-dependent module and function analyses together.
    #[test]
    fn test_invalidate_dispatch_dependencies() {
        let test = TestProgram::new(
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
        let mut cache = AnalysisCache::new();
        let control = cache.control(function, &test.tree);
        let resolution = cache.resolution(&test.tree, &test.dispatch);
        let origin = cache.origin(function, &test.tree, &resolution);
        let calls = cache.call(&test.tree, &test.dispatch);
        let effects = cache.effect(&test.tree, &test.accesses, &test.effects, &test.dispatch);

        // invalidate the dispatch table and its dependent analyses
        cache.invalidate(Mutation::DISPATCH);
        let new_resolution = cache.resolution(&test.tree, &test.dispatch);
        assert!(!Arc::ptr_eq(&resolution, &new_resolution));
        assert!(!Arc::ptr_eq(
            &origin,
            &cache.origin(function, &test.tree, &new_resolution)
        ));
        assert!(!Arc::ptr_eq(
            &calls,
            &cache.call(&test.tree, &test.dispatch)
        ));
        assert!(!Arc::ptr_eq(
            &effects,
            &cache.effect(&test.tree, &test.accesses, &test.effects, &test.dispatch)
        ));

        // check that dispatch changes preserve the control graph
        assert!(Arc::ptr_eq(&control, &cache.control(function, &test.tree)));
    }
}
