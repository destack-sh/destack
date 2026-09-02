use std::sync::Arc;

use crate as mir;
use crate::EffectTable;

use super::{
    AliasTable, CallTable, ConstantTable, ControlTable, CostTable, CostWeights, DefinitionTable,
    DominatorTable, EscapeTable, EvolutionTable, ExpressionTable, FrequencyTable,
    InitializationTable, LinkTable, LivenessTable, LoopTable, MemoryTable, MoveTable, Mutation,
    OriginTable, PlaceTable, PostdominatorTable, RangeTable, ResolutionTable, UseTable,
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
    /// The cached operation costs.
    cost: Option<Arc<CostTable>>,
    /// The cached value definitions.
    definition: Option<Arc<DefinitionTable>>,
    /// The cached dominators.
    dominator: Option<Arc<DominatorTable>>,
    /// The cached escapes.
    escape: Option<Arc<EscapeTable>>,
    /// The cached available expressions.
    expression: Option<Arc<ExpressionTable>>,
    /// The cached execution frequencies.
    frequency: Option<Arc<FrequencyTable>>,
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
    /// The cached value ranges.
    range: Option<Arc<RangeTable>>,
    /// The cached scalar evolution.
    evolution: Option<Arc<EvolutionTable>>,
    /// The cached value uses.
    uses: Option<Arc<UseTable>>,
    /// The analysis options.
    options: Arc<AnalysisOptions>,
}

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
            let analyses = &mut self.functions[index].1;
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
            cost: None,
            definition: None,
            dominator: None,
            escape: None,
            expression: None,
            frequency: None,
            initialization: None,
            liveness: None,
            loops: None,
            memory: None,
            moves: None,
            place: None,
            postdominator: None,
            origin: None,
            range: None,
            evolution: None,
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
    function_analysis!(cost, cost, CostTable, "Return the cost model.");
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
    function_analysis!(
        expression,
        expression,
        ExpressionTable,
        "Return available expressions."
    );
    function_analysis!(
        frequency,
        frequency,
        FrequencyTable,
        "Return execution frequencies."
    );
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
    function_analysis!(range, range, RangeTable, "Return value range.");
    function_analysis!(
        evolution,
        evolution,
        EvolutionTable,
        "Return scalar evolution."
    );
    function_analysis!(uses, uses, UseTable, "Return value uses.");

    /// Drop every analysis the given mutation invalidates.
    pub fn invalidate(&mut self, mutation: Mutation) {
        if AliasTable::INVALIDATED_BY.intersects(mutation) {
            self.alias = None;
        }
        if ConstantTable::INVALIDATED_BY.intersects(mutation) {
            self.constant = None;
        }
        if ControlTable::INVALIDATED_BY.intersects(mutation) {
            self.control = None;
        }
        if CostTable::INVALIDATED_BY.intersects(mutation) {
            self.cost = None;
        }
        if DefinitionTable::INVALIDATED_BY.intersects(mutation) {
            self.definition = None;
        }
        if DominatorTable::INVALIDATED_BY.intersects(mutation) {
            self.dominator = None;
        }
        if EscapeTable::INVALIDATED_BY.intersects(mutation) {
            self.escape = None;
        }
        if ExpressionTable::INVALIDATED_BY.intersects(mutation) {
            self.expression = None;
        }
        if FrequencyTable::INVALIDATED_BY.intersects(mutation) {
            self.frequency = None;
        }
        if LivenessTable::INVALIDATED_BY.intersects(mutation) {
            self.liveness = None;
        }
        if InitializationTable::INVALIDATED_BY.intersects(mutation) {
            self.initialization = None;
        }
        if LoopTable::INVALIDATED_BY.intersects(mutation) {
            self.loops = None;
        }
        if MemoryTable::INVALIDATED_BY.intersects(mutation) {
            self.memory = None;
        }
        if MoveTable::INVALIDATED_BY.intersects(mutation) {
            self.moves = None;
        }
        if PlaceTable::INVALIDATED_BY.intersects(mutation) {
            self.place = None;
        }
        if PostdominatorTable::INVALIDATED_BY.intersects(mutation) {
            self.postdominator = None;
        }
        if OriginTable::INVALIDATED_BY.intersects(mutation) {
            self.origin = None;
        }
        if RangeTable::INVALIDATED_BY.intersects(mutation) {
            self.range = None;
        }
        if EvolutionTable::INVALIDATED_BY.intersects(mutation) {
            self.evolution = None;
        }
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
    function_analysis_through_module!(cost, CostTable, "Return the function cost model.");
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
    function_analysis_through_module!(
        expression,
        ExpressionTable,
        "Return function available expressions."
    );
    function_analysis_through_module!(
        frequency,
        FrequencyTable,
        "Return function execution frequencies."
    );
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
    function_analysis_through_module!(range, RangeTable, "Return function value range.");
    function_analysis_through_module!(
        evolution,
        EvolutionTable,
        "Return function scalar evolution."
    );
    function_analysis_through_module!(
        initialization,
        InitializationTable,
        "Return function move-path initialization."
    );
    function_analysis_through_module!(uses, UseTable, "Return function value uses.");

    /// Drop every analysis the given mutation invalidates.
    pub fn invalidate(&mut self, mutation: Mutation) {
        if CallTable::INVALIDATED_BY.intersects(mutation) {
            self.call = None;
        }
        if ResolutionTable::INVALIDATED_BY.intersects(mutation) {
            self.resolution = None;
        }
        if EffectTable::INVALIDATED_BY.intersects(mutation) {
            self.effect = None;
        }
        if LinkTable::INVALIDATED_BY.intersects(mutation) {
            self.link = None;
        }

        for (_, analyses) in &mut self.functions {
            analyses.invalidate(mutation);
        }
    }
}

impl Default for AnalysisCache {
    fn default() -> Self {
        Self::new()
    }
}
