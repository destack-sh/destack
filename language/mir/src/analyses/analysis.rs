use std::collections::HashMap;
use std::sync::Arc;

use crate as mir;

use super::{
    AliasAnalysis, CallGraph, ConstantPropagation, ControlFlowGraph, CostModel, CostWeights,
    DispatchAnalysis, DominatorTree, EscapeAnalysis, ExecutionFrequency, FunctionEffectAnalysis,
    FunctionLiveness, LifetimeAnalysis, LinkGraph, LoopAnalysis, MemorySSA, Mutation,
    RangeAnalysis, ScalarEvolution, ValueDefinitions, ValueTypes, ValueUses,
};

/// Largest loop scale for profile frequency analysis.
const DEFAULT_MAX_LOOP_SCALE: f64 = 4096.0;

/// Minimum execution count for a hot operation.
const DEFAULT_HOT_COUNT: u64 = 50;

/// Maximum execution count for a cold operation.
const DEFAULT_COLD_COUNT: u64 = 8;

/// Minimum caller relative count for a hot operation.
const DEFAULT_HOT_RATIO: f64 = 0.10;

/// Maximum caller relative count for a cold operation.
const DEFAULT_COLD_RATIO: f64 = 0.01;

/// Range refinement iterations before widening.
const DEFAULT_RANGE_WIDEN_THRESHOLD: u32 = 32;

/// Options for MIR execution frequency analysis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExecutionFrequencyOptions {
    /// Maximum loop scale for near-certain back edges.
    pub max_loop_scale: f64,
}

impl Default for ExecutionFrequencyOptions {
    fn default() -> Self {
        Self {
            max_loop_scale: DEFAULT_MAX_LOOP_SCALE,
        }
    }
}

/// Thresholds for classifying profiled operations as hot or cold.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HotnessThresholds {
    /// Minimum execution count for a hot operation.
    pub hot_count: u64,
    /// Maximum execution count for a cold operation.
    pub cold_count: u64,
    /// Minimum caller-relative count for a hot operation.
    pub hot_ratio: f64,
    /// Maximum caller-relative count for a cold operation.
    pub cold_ratio: f64,
}

impl Default for HotnessThresholds {
    fn default() -> Self {
        Self {
            hot_count: DEFAULT_HOT_COUNT,
            cold_count: DEFAULT_COLD_COUNT,
            hot_ratio: DEFAULT_HOT_RATIO,
            cold_ratio: DEFAULT_COLD_RATIO,
        }
    }
}

/// Options for MIR range analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeOptions {
    /// Block refinement iterations before widening.
    pub widen_threshold: u32,
}

impl Default for RangeOptions {
    fn default() -> Self {
        Self {
            widen_threshold: DEFAULT_RANGE_WIDEN_THRESHOLD,
        }
    }
}

/// Options used by MIR analyses.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct AnalysisOptions {
    /// Target layout for layout sensitive analyses.
    pub target_layout: mir::TargetLayout,
    /// Cost weights for MIR cost analysis.
    pub cost_weights: CostWeights,
    /// Options for MIR execution frequency analysis.
    pub execution_frequency: ExecutionFrequencyOptions,
    /// Thresholds for profile hotness classification.
    pub hotness: HotnessThresholds,
    /// Options for MIR range analysis.
    pub range: RangeOptions,
}

impl AnalysisOptions {
    /// Create MIR analysis options.
    pub fn new(target_layout: mir::TargetLayout) -> Self {
        Self {
            target_layout,
            cost_weights: CostWeights::default(),
            execution_frequency: ExecutionFrequencyOptions::default(),
            hotness: HotnessThresholds::default(),
            range: RangeOptions::default(),
        }
    }

    /// Create MIR analysis options with cost weights.
    pub fn with_cost_weights(mut self, cost_weights: CostWeights) -> Self {
        self.cost_weights = cost_weights;

        self
    }

    /// Create MIR analysis options with execution frequency options.
    pub fn with_execution_frequency_options(
        mut self,
        execution_frequency: ExecutionFrequencyOptions,
    ) -> Self {
        self.execution_frequency = execution_frequency;

        self
    }

    /// Create MIR analysis options with hotness thresholds.
    pub fn with_hotness_thresholds(mut self, hotness: HotnessThresholds) -> Self {
        self.hotness = hotness;

        self
    }

    /// Create MIR analysis options with range options.
    pub fn with_range_options(mut self, range: RangeOptions) -> Self {
        self.range = range;

        self
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
            let analyses = self.functions.entry(function_id).or_insert_with(|| {
                FunctionAnalyses::with_options(self.options)
            });
            let function = tree.get(function_id);

            analyses.$method(function, tree, $($parameter,)*)
        }
    };
}

/// Function analyses across one MIR pass sequence.
#[derive(Debug)]
pub struct FunctionAnalyses {
    alias: Option<Arc<AliasAnalysis>>,
    constants: Option<Arc<ConstantPropagation>>,
    control_flow: Option<Arc<ControlFlowGraph>>,
    cost: Option<Arc<CostModel>>,
    dominators: Option<Arc<DominatorTree>>,
    escape: Option<Arc<EscapeAnalysis>>,
    frequency: Option<Arc<ExecutionFrequency>>,
    liveness: Option<Arc<FunctionLiveness>>,
    loops: Option<Arc<LoopAnalysis>>,
    memory_ssa: Option<Arc<MemorySSA>>,
    ranges: Option<Arc<RangeAnalysis>>,
    scalar_evolution: Option<Arc<ScalarEvolution>>,
    value_definitions: Option<Arc<ValueDefinitions>>,
    value_types: Option<Arc<ValueTypes>>,
    value_uses: Option<Arc<ValueUses>>,
    options: AnalysisOptions,
}

impl FunctionAnalyses {
    /// Create empty function analyses.
    pub fn new() -> Self {
        Self::with_options(AnalysisOptions::default())
    }

    /// Create empty function analyses with the given options.
    pub fn with_options(options: AnalysisOptions) -> Self {
        Self {
            alias: None,
            constants: None,
            control_flow: None,
            cost: None,
            dominators: None,
            escape: None,
            frequency: None,
            liveness: None,
            loops: None,
            memory_ssa: None,
            ranges: None,
            scalar_evolution: None,
            value_definitions: None,
            value_types: None,
            value_uses: None,
            options,
        }
    }

    /// Get the analysis options.
    pub fn options(&self) -> &AnalysisOptions {
        &self.options
    }

    /// Return the target layout for this analysis run.
    pub fn target_layout(&self) -> mir::TargetLayout {
        self.options.target_layout
    }

    function_analysis!(alias, alias, AliasAnalysis, "Return alias analysis.");
    function_analysis!(
        constants,
        constants,
        ConstantPropagation,
        "Return constant propagation."
    );
    function_analysis!(
        control_flow,
        control_flow,
        ControlFlowGraph,
        "Return the control-flow graph."
    );
    function_analysis!(cost, cost, CostModel, "Return the cost model.");
    function_analysis!(
        dominators,
        dominators,
        DominatorTree,
        "Return the dominator tree."
    );
    function_analysis!(escape, escape, EscapeAnalysis, "Return escape analysis.");
    function_analysis!(
        frequency,
        frequency,
        ExecutionFrequency,
        "Return execution frequencies."
    );
    function_analysis!(
        liveness,
        liveness,
        FunctionLiveness,
        "Return value liveness."
    );
    function_analysis!(loops, loops, LoopAnalysis, "Return loop analysis.");
    function_analysis!(
        memory_ssa,
        memory_ssa,
        MemorySSA,
        "Return memory SSA.",
        memory: &mir::MemoryTable,
        effects: &mir::EffectTable
    );
    function_analysis!(ranges, ranges, RangeAnalysis, "Return value ranges.");
    function_analysis!(
        scalar_evolution,
        scalar_evolution,
        ScalarEvolution,
        "Return scalar evolution."
    );
    function_analysis!(
        value_definitions,
        value_definitions,
        ValueDefinitions,
        "Return value definitions."
    );
    function_analysis!(value_types, value_types, ValueTypes, "Return value types.");
    function_analysis!(value_uses, value_uses, ValueUses, "Return value uses.");

    /// Drop every analysis the given mutation invalidates.
    pub fn invalidate(&mut self, mutation: Mutation) {
        if AliasAnalysis::INVALIDATED_BY.intersects(mutation) {
            self.alias = None;
        }
        if ConstantPropagation::INVALIDATED_BY.intersects(mutation) {
            self.constants = None;
        }
        if ControlFlowGraph::INVALIDATED_BY.intersects(mutation) {
            self.control_flow = None;
        }
        if CostModel::INVALIDATED_BY.intersects(mutation) {
            self.cost = None;
        }
        if DominatorTree::INVALIDATED_BY.intersects(mutation) {
            self.dominators = None;
        }
        if EscapeAnalysis::INVALIDATED_BY.intersects(mutation) {
            self.escape = None;
        }
        if ExecutionFrequency::INVALIDATED_BY.intersects(mutation) {
            self.frequency = None;
        }
        if FunctionLiveness::INVALIDATED_BY.intersects(mutation) {
            self.liveness = None;
        }
        if LoopAnalysis::INVALIDATED_BY.intersects(mutation) {
            self.loops = None;
        }
        if MemorySSA::INVALIDATED_BY.intersects(mutation) {
            self.memory_ssa = None;
        }
        if RangeAnalysis::INVALIDATED_BY.intersects(mutation) {
            self.ranges = None;
        }
        if ScalarEvolution::INVALIDATED_BY.intersects(mutation) {
            self.scalar_evolution = None;
        }
        if ValueDefinitions::INVALIDATED_BY.intersects(mutation) {
            self.value_definitions = None;
        }
        if ValueTypes::INVALIDATED_BY.intersects(mutation) {
            self.value_types = None;
        }
        if ValueUses::INVALIDATED_BY.intersects(mutation) {
            self.value_uses = None;
        }
    }
}

impl Default for FunctionAnalyses {
    fn default() -> Self {
        Self::new()
    }
}

/// Module analyses across one MIR pass sequence.
#[derive(Debug)]
pub struct ModuleAnalyses {
    call_graph: Option<Arc<CallGraph>>,
    dispatch: Option<Arc<DispatchAnalysis>>,
    function_effects: Option<Arc<FunctionEffectAnalysis>>,
    lifetimes: Option<Arc<LifetimeAnalysis>>,
    link_graph: Option<Arc<LinkGraph>>,
    functions: HashMap<mir::FunctionId, FunctionAnalyses>,
    options: AnalysisOptions,
}

impl ModuleAnalyses {
    /// Create empty module analyses.
    pub fn new() -> Self {
        Self::with_options(AnalysisOptions::default())
    }

    /// Create empty module analyses with the given options.
    pub fn with_options(options: AnalysisOptions) -> Self {
        Self {
            call_graph: None,
            dispatch: None,
            function_effects: None,
            lifetimes: None,
            link_graph: None,
            functions: HashMap::new(),
            options,
        }
    }

    /// Get the analysis options.
    pub fn options(&self) -> &AnalysisOptions {
        &self.options
    }

    module_analysis!(
        call_graph,
        call_graph,
        CallGraph,
        "Return the call graph.",
        effects: &mir::EffectTable
    );
    module_analysis!(
        dispatch,
        dispatch,
        DispatchAnalysis,
        "Return dispatch analysis.",
        dispatch_table: &mir::DispatchTable
    );
    module_analysis!(
        function_effects,
        function_effects,
        FunctionEffectAnalysis,
        "Return function effects.",
        memory: &mir::MemoryTable,
        effects: &mir::EffectTable
    );
    module_analysis!(lifetimes, lifetimes, LifetimeAnalysis, "Return lifetimes.");
    module_analysis!(
        link_graph,
        link_graph,
        LinkGraph,
        "Return the link graph.",
        effects: &mir::EffectTable
    );

    function_analysis_through_module!(alias, AliasAnalysis, "Return function alias analysis.");
    function_analysis_through_module!(
        constants,
        ConstantPropagation,
        "Return function constant propagation."
    );
    function_analysis_through_module!(
        control_flow,
        ControlFlowGraph,
        "Return the function control-flow graph."
    );
    function_analysis_through_module!(cost, CostModel, "Return the function cost model.");
    function_analysis_through_module!(
        dominators,
        DominatorTree,
        "Return the function dominator tree."
    );
    function_analysis_through_module!(escape, EscapeAnalysis, "Return function escape analysis.");
    function_analysis_through_module!(
        frequency,
        ExecutionFrequency,
        "Return function execution frequencies."
    );
    function_analysis_through_module!(
        liveness,
        FunctionLiveness,
        "Return function value liveness."
    );
    function_analysis_through_module!(loops, LoopAnalysis, "Return function loop analysis.");
    function_analysis_through_module!(
        memory_ssa,
        MemorySSA,
        "Return function memory SSA.",
        memory: &mir::MemoryTable,
        effects: &mir::EffectTable
    );
    function_analysis_through_module!(ranges, RangeAnalysis, "Return function value ranges.");
    function_analysis_through_module!(
        scalar_evolution,
        ScalarEvolution,
        "Return function scalar evolution."
    );
    function_analysis_through_module!(
        value_definitions,
        ValueDefinitions,
        "Return function value definitions."
    );
    function_analysis_through_module!(value_types, ValueTypes, "Return function value types.");
    function_analysis_through_module!(value_uses, ValueUses, "Return function value uses.");

    /// Drop every analysis the given mutation invalidates.
    pub fn invalidate(&mut self, mutation: Mutation) {
        if CallGraph::INVALIDATED_BY.intersects(mutation) {
            self.call_graph = None;
        }
        if DispatchAnalysis::INVALIDATED_BY.intersects(mutation) {
            self.dispatch = None;
        }
        if FunctionEffectAnalysis::INVALIDATED_BY.intersects(mutation) {
            self.function_effects = None;
        }
        if LifetimeAnalysis::INVALIDATED_BY.intersects(mutation) {
            self.lifetimes = None;
        }
        if LinkGraph::INVALIDATED_BY.intersects(mutation) {
            self.link_graph = None;
        }

        for analyses in self.functions.values_mut() {
            analyses.invalidate(mutation);
        }
    }
}

impl Default for ModuleAnalyses {
    fn default() -> Self {
        Self::new()
    }
}
