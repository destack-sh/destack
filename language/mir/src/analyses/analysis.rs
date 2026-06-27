use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use crate as mir;

use super::{CostWeights, Mutation};

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

/// Unique identifier for an analysis type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnalysisId(pub &'static str);

impl std::fmt::Display for AnalysisId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Base trait for all analyses.
///
/// Provides identity and invalidation information for caching. Concrete analyses
/// implement `FunctionAnalysis`, `ModuleAnalysis`, or a consumer defined scope
/// trait built on the same identity scheme.
pub trait Analysis: 'static + Send + Sync + Sized {
    /// Unique identifier for this analysis.
    const ID: AnalysisId;

    /// The mutations that invalidate this analysis.
    const INVALIDATED_BY: Mutation = Mutation::ALL;
}

/// Function-scoped analysis.
pub trait FunctionAnalysis: Analysis {
    /// Compute this analysis for a function.
    fn compute(
        function: &mir::Function,
        tree: &mir::Tree,
        analyses: &FunctionAnalysisCache,
    ) -> Self;
}

/// Module-scoped analysis.
pub trait ModuleAnalysis: Analysis {
    /// Compute this analysis for the module.
    fn compute(tree: &mir::Tree, analyses: &TreeAnalysisCache) -> Self;
}

/// Shared cache machinery for analyses of any scope.
pub(crate) struct AnalysisCache {
    cache: RefCell<HashMap<AnalysisId, (Arc<dyn Any + Send + Sync>, Mutation)>>,
}

impl AnalysisCache {
    /// Create an empty cache.
    pub(crate) fn new() -> Self {
        Self {
            cache: RefCell::new(HashMap::new()),
        }
    }

    /// Return a cached analysis or compute it with `compute` and cache it.
    pub(crate) fn get_or_compute<A: Analysis>(&self, compute: impl FnOnce() -> A) -> Arc<A> {
        // return the cached result when present
        if let Some((cached, _)) = self.cache.borrow().get(&A::ID) {
            return match cached.clone().downcast::<A>() {
                Ok(cached) => cached,
                Err(_) => unreachable!("analysis cache type mismatch for {}", A::ID),
            };
        }

        // compute (may recursively query the cache for dependencies)
        let result = Arc::new(compute());

        // cache it next to the mutations that invalidate it
        self.cache
            .borrow_mut()
            .insert(A::ID, (result.clone(), A::INVALIDATED_BY));
        result
    }

    /// Check if an analysis is cached.
    pub(crate) fn is_cached<A: Analysis>(&self) -> bool {
        self.cache.borrow().contains_key(&A::ID)
    }

    /// Drop every cached analysis the given mutation invalidates.
    pub(crate) fn apply(&self, mutation: Mutation) {
        // nothing changed; every analysis stays valid
        if mutation.is_none() {
            return;
        }

        // keep only analyses the mutation does not touch
        self.cache
            .borrow_mut()
            .retain(|_, (_, invalidated_by)| !invalidated_by.intersects(mutation));
    }

    /// The number of analyses currently cached.
    pub(crate) fn cached_count(&self) -> usize {
        self.cache.borrow().len()
    }
}

impl Default for AnalysisCache {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for AnalysisCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalysisCache")
            .field("cached_count", &self.cached_count())
            .finish()
    }
}

/// Caches function-scoped analyses for one immutable table snapshot.
///
/// A pipeline creates a fresh cache from the current MIR tables before a pass runs.
#[derive(Debug)]
pub struct FunctionAnalysisCache {
    cache: AnalysisCache,
    options: AnalysisOptions,
    memory: mir::MemoryTable,
    effects: mir::EffectTable,
}

impl FunctionAnalysisCache {
    /// Create a new function analysis cache.
    pub fn new(memory: &mir::MemoryTable, effects: &mir::EffectTable) -> Self {
        Self::with_options(AnalysisOptions::default(), memory, effects)
    }

    /// Create a new function analysis cache with the given options.
    pub fn with_options(
        options: AnalysisOptions,
        memory: &mir::MemoryTable,
        effects: &mir::EffectTable,
    ) -> Self {
        Self {
            cache: AnalysisCache::new(),
            options,
            memory: memory.clone(),
            effects: effects.clone(),
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

    /// Get the explicit memory table.
    pub fn memory(&self) -> &mir::MemoryTable {
        &self.memory
    }

    /// Get the effect table.
    pub fn effects(&self) -> &mir::EffectTable {
        &self.effects
    }

    /// Get or compute a function analysis for the given function.
    pub fn get<A: FunctionAnalysis>(&self, function: &mir::Function, tree: &mir::Tree) -> Arc<A> {
        self.cache
            .get_or_compute(|| A::compute(function, tree, self))
    }

    /// Check if an analysis is cached.
    pub fn is_cached<A: FunctionAnalysis>(&self) -> bool {
        self.cache.is_cached::<A>()
    }

    /// Drop every analysis the given mutation invalidates.
    pub fn apply(&self, mutation: Mutation) {
        self.cache.apply(mutation);
    }
}

/// Caches tree-scoped analyses across one MIR pass sequence.
#[derive(Debug)]
pub struct TreeAnalysisCache {
    cache: AnalysisCache,
    functions: RefCell<HashMap<mir::FunctionId, Rc<FunctionAnalysisCache>>>,
    options: AnalysisOptions,
    dispatch: mir::DispatchTable,
    memory: mir::MemoryTable,
    effects: mir::EffectTable,
}

impl TreeAnalysisCache {
    /// Create a new tree analysis cache.
    pub fn new(
        dispatch: &mir::DispatchTable,
        memory: &mir::MemoryTable,
        effects: &mir::EffectTable,
    ) -> Self {
        Self::with_options(AnalysisOptions::default(), dispatch, memory, effects)
    }

    /// Create a new tree analysis cache with the given options.
    pub fn with_options(
        options: AnalysisOptions,
        dispatch: &mir::DispatchTable,
        memory: &mir::MemoryTable,
        effects: &mir::EffectTable,
    ) -> Self {
        Self {
            cache: AnalysisCache::new(),
            functions: RefCell::new(HashMap::new()),
            options,
            dispatch: dispatch.clone(),
            memory: memory.clone(),
            effects: effects.clone(),
        }
    }

    /// Get the analysis options.
    pub fn options(&self) -> &AnalysisOptions {
        &self.options
    }

    /// Get the dispatch table.
    pub fn dispatch(&self) -> &mir::DispatchTable {
        &self.dispatch
    }

    /// Get the explicit memory table.
    pub fn memory(&self) -> &mir::MemoryTable {
        &self.memory
    }

    /// Get the effect table.
    pub fn effects(&self) -> &mir::EffectTable {
        &self.effects
    }

    /// Get or compute a tree analysis for the given tree.
    pub fn get<A: ModuleAnalysis>(&self, tree: &mir::Tree) -> Arc<A> {
        self.cache.get_or_compute(|| A::compute(tree, self))
    }

    /// Get or compute a function analysis through this module cache.
    pub fn get_function<A: FunctionAnalysis>(
        &self,
        function_id: mir::FunctionId,
        tree: &mir::Tree,
    ) -> Arc<A> {
        let analyses = self
            .functions
            .borrow_mut()
            .entry(function_id)
            .or_insert_with(|| {
                Rc::new(FunctionAnalysisCache::with_options(
                    self.options,
                    &self.memory,
                    &self.effects,
                ))
            })
            .clone();
        let function = tree.get(function_id);

        analyses.get::<A>(function, tree)
    }

    /// Check if an analysis is cached.
    pub fn is_cached<A: ModuleAnalysis>(&self) -> bool {
        self.cache.is_cached::<A>()
    }

    /// Drop every analysis the given mutation invalidates.
    pub fn apply(&self, mutation: Mutation) {
        self.cache.apply(mutation);
        for analyses in self.functions.borrow().values() {
            analyses.apply(mutation);
        }
    }
}
