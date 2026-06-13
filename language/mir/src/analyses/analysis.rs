use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use crate as mir;

use super::{
    AliasAnalysis, AvailableExpressions, CallGraph, CallTargetAnalysis, ConstantPropagation,
    ControlFlowGraph, DominatorTree, FunctionLiveness, LifetimeAnalysis, LoopAnalysis, MemorySSA,
    PostDominatorTree, RangeAnalysis, ReachingDefinitions, ScalarEvolution,
};

/// Type related context for MIR analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeContext {
    /// Pointer width in bits for pointer sized integers.
    pub pointer_width_bits: u16,
}

impl Default for TypeContext {
    fn default() -> Self {
        Self {
            pointer_width_bits: usize::BITS as u16,
        }
    }
}

/// Options used by MIR analyses.
#[derive(Debug, Clone, Copy, Default)]
pub struct MirAnalysisOptions {
    /// Enable strict borrow checking.
    pub strict_borrow_mode: bool,
    /// Type context for layout sensitive analyses.
    pub type_context: TypeContext,
}

impl MirAnalysisOptions {
    /// Create MIR analysis options.
    pub fn new(strict_borrow_mode: bool, type_context: TypeContext) -> Self {
        Self {
            strict_borrow_mode,
            type_context,
        }
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
/// Provides identity and dependency information for caching and invalidation.
/// Concrete analyses implement `FunctionAnalysis`, `ModuleAnalysis`, or a
/// consumer defined scope trait built on the same identity scheme.
pub trait Analysis: 'static + Send + Sync + Sized {
    /// Unique identifier for this analysis.
    const ID: AnalysisId;

    /// Analyses this one depends on (for cascading invalidation).
    ///
    /// When a dependency is invalidated, this analysis is also invalidated.
    const DEPENDENCIES: &'static [AnalysisId] = &[];
}

/// Function-scoped analysis.
pub trait FunctionAnalysis: Analysis {
    /// Compute this analysis for a function.
    fn compute(function: &mir::Function, tree: &mir::Tree, analyses: &FunctionAnalyses) -> Self;
}

/// Module-scoped analysis.
pub trait ModuleAnalysis: Analysis {
    /// Compute this analysis for the module.
    fn compute(tree: &mir::Tree, analyses: &ModuleAnalyses) -> Self;
}

/// Dependency graph for cascading invalidation.
#[derive(Debug, Default)]
pub struct DependencyGraph {
    /// Maps each analysis to analyses that depend on it.
    dependents: HashMap<AnalysisId, Vec<AnalysisId>>,
}

impl DependencyGraph {
    /// Register an analysis and its dependencies.
    pub fn register(&mut self, id: AnalysisId, dependencies: &[AnalysisId]) {
        for dep in dependencies {
            self.dependents.entry(*dep).or_default().push(id);
        }
    }

    /// Get all analyses that directly depend on the given analysis.
    pub fn get_dependents(&self, id: AnalysisId) -> &[AnalysisId] {
        self.dependents
            .get(&id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

/// Register an analysis in the dependency graph.
pub fn register_analysis<A: Analysis>(graph: &mut DependencyGraph) {
    graph.register(A::ID, A::DEPENDENCIES);
}

/// Shared cache machinery for analyses of any scope.
///
/// Holds computed analyses keyed by id alongside the dependency graph used for
/// cascading invalidation. The cache never owns the IR or workset it analyzes:
/// the scope wrappers ([`FunctionAnalyses`], [`ModuleAnalyses`], and the package
/// and program caches in the optimizer) supply the work input at each query, so
/// the cache outlives any single borrow and survives the mutations passes
/// perform between queries.
pub struct AnalysisCache {
    cache: RefCell<HashMap<AnalysisId, Arc<dyn Any + Send + Sync>>>,
    dependency_graph: DependencyGraph,
}

impl AnalysisCache {
    /// Create a cache whose dependency graph is populated by `register`.
    pub fn new(register: impl FnOnce(&mut DependencyGraph)) -> Self {
        let mut dependency_graph = DependencyGraph::default();
        register(&mut dependency_graph);

        Self {
            cache: RefCell::new(HashMap::new()),
            dependency_graph,
        }
    }

    /// Return a cached analysis or compute it with `compute` and cache it.
    pub fn get_or_compute<A: Analysis>(&self, compute: impl FnOnce() -> A) -> Arc<A> {
        // return the cached result when present
        if let Some(cached) = self.cache.borrow().get(&A::ID).cloned() {
            return cached.downcast::<A>().expect("analysis type mismatch");
        }

        // compute (may recursively query the cache for dependencies)
        let result = Arc::new(compute());

        // cache and return
        self.cache.borrow_mut().insert(A::ID, result.clone());
        result
    }

    /// Check if an analysis is cached.
    pub fn is_cached<A: Analysis>(&self) -> bool {
        self.cache.borrow().contains_key(&A::ID)
    }

    /// Invalidate an analysis and all its dependents (cascading).
    pub fn invalidate(&self, id: AnalysisId) {
        // remove from cache
        self.cache.borrow_mut().remove(&id);

        // cascade to dependents
        for dependent in self.dependency_graph.get_dependents(id) {
            self.invalidate(*dependent);
        }
    }

    /// Clear all cached analyses (when moving to a new work unit).
    pub fn clear(&self) {
        self.cache.borrow_mut().clear();
    }

    /// Invalidate every analysis a pass did not preserve.
    pub fn apply_preservation(&self, preserved: &AnalysisPreservation) {
        match preserved {
            // everything stays valid
            AnalysisPreservation::All => {}
            // nothing preserved, clear all
            AnalysisPreservation::Some(ids) if ids.is_empty() => self.clear(),
            // remove anything not preserved
            AnalysisPreservation::Some(preserved_ids) => {
                let to_remove: Vec<_> = self
                    .cache
                    .borrow()
                    .keys()
                    .filter(|id| !preserved_ids.contains(id))
                    .copied()
                    .collect();

                for id in to_remove {
                    self.invalidate(id);
                }
            }
        }
    }

    /// The number of analyses currently cached.
    pub fn cached_count(&self) -> usize {
        self.cache.borrow().len()
    }
}

impl std::fmt::Debug for AnalysisCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnalysisCache")
            .field("cached_count", &self.cached_count())
            .finish()
    }
}

/// Caches function-scoped analyses across one function's pass sequence.
///
/// A pipeline holds one cache per function, clears it when moving to the next
/// function, and applies each pass's preservation to invalidate stale entries.
#[derive(Debug)]
pub struct FunctionAnalyses {
    cache: AnalysisCache,
    options: MirAnalysisOptions,
}

impl FunctionAnalyses {
    /// Create a new function analysis cache with default options.
    pub fn new() -> Self {
        Self::with_options(MirAnalysisOptions::default())
    }

    /// Create a new function analysis cache with the given options.
    pub fn with_options(options: MirAnalysisOptions) -> Self {
        Self {
            cache: AnalysisCache::new(Self::register),
            options,
        }
    }

    /// Register all known function analyses.
    fn register(graph: &mut DependencyGraph) {
        register_analysis::<ControlFlowGraph>(graph);
        register_analysis::<DominatorTree>(graph);
        register_analysis::<PostDominatorTree>(graph);
        register_analysis::<LoopAnalysis>(graph);
        register_analysis::<FunctionLiveness>(graph);
        register_analysis::<ConstantPropagation>(graph);
        register_analysis::<ReachingDefinitions>(graph);
        register_analysis::<AvailableExpressions>(graph);
        register_analysis::<ScalarEvolution>(graph);
        register_analysis::<RangeAnalysis>(graph);
        register_analysis::<AliasAnalysis>(graph);
        register_analysis::<MemorySSA>(graph);
    }

    /// Get the analysis options.
    pub fn options(&self) -> &MirAnalysisOptions {
        &self.options
    }

    /// Return the type context for this analysis run.
    pub fn type_context(&self) -> TypeContext {
        self.options.type_context
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

    /// Invalidate an analysis and all its dependents (cascading).
    pub fn invalidate(&self, id: AnalysisId) {
        self.cache.invalidate(id);
    }

    /// Clear all cached analyses (when moving to a new function).
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// Invalidate every analysis a pass did not preserve.
    pub fn apply_preservation(&self, preserved: &AnalysisPreservation) {
        self.cache.apply_preservation(preserved);
    }
}

impl Default for FunctionAnalyses {
    fn default() -> Self {
        Self::new()
    }
}

/// Caches module-scoped analyses across one module's pass sequence.
#[derive(Debug)]
pub struct ModuleAnalyses {
    cache: AnalysisCache,
}

impl ModuleAnalyses {
    /// Create a new module analysis cache.
    pub fn new() -> Self {
        Self {
            cache: AnalysisCache::new(Self::register),
        }
    }

    /// Register all known module analyses.
    fn register(graph: &mut DependencyGraph) {
        register_analysis::<LifetimeAnalysis>(graph);
        register_analysis::<CallGraph>(graph);
        register_analysis::<CallTargetAnalysis>(graph);
    }

    /// Get or compute a module analysis for the given tree.
    pub fn get<A: ModuleAnalysis>(&self, tree: &mir::Tree) -> Arc<A> {
        self.cache.get_or_compute(|| A::compute(tree, self))
    }

    /// Check if an analysis is cached.
    pub fn is_cached<A: ModuleAnalysis>(&self) -> bool {
        self.cache.is_cached::<A>()
    }

    /// Invalidate a module analysis and all its dependents.
    pub fn invalidate(&self, id: AnalysisId) {
        self.cache.invalidate(id);
    }

    /// Clear all cached analyses.
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// Invalidate every analysis a pass did not preserve.
    pub fn apply_preservation(&self, preserved: &AnalysisPreservation) {
        self.cache.apply_preservation(preserved);
    }
}

impl Default for ModuleAnalyses {
    fn default() -> Self {
        Self::new()
    }
}

/// Declares what analyses are still valid after a pass runs.
///
/// Returned dynamically by passes based on what they actually changed.
#[derive(Debug, Clone)]
pub enum AnalysisPreservation {
    /// All analyses preserved (pass made no changes).
    All,
    /// Only specific analyses preserved.
    Some(Vec<AnalysisId>),
}

impl AnalysisPreservation {
    /// All analyses preserved.
    pub fn all() -> Self {
        Self::All
    }

    /// No analyses preserved.
    pub fn none() -> Self {
        Self::Some(Vec::new())
    }

    /// Specific analyses preserved.
    pub fn preserving(ids: &[AnalysisId]) -> Self {
        Self::Some(ids.to_vec())
    }

    /// Check if all analyses are preserved.
    pub fn preserves_all(&self) -> bool {
        matches!(self, Self::All)
    }

    /// Check if a specific analysis is preserved.
    pub fn is_preserved(&self, id: AnalysisId) -> bool {
        match self {
            Self::All => true,
            Self::Some(ids) => ids.contains(&id),
        }
    }
}

impl Default for AnalysisPreservation {
    fn default() -> Self {
        Self::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ensure the dependency registry matches each analysis declaration.
    #[test]
    fn test_function_analysis_dependency_registry() {
        // build the registered dependency graph
        let analyses = FunctionAnalyses::new();
        let graph = &analyses.cache.dependency_graph;

        // validate dependency edges for all registered analyses
        assert_registered_dependencies::<ControlFlowGraph>(graph);
        assert_registered_dependencies::<DominatorTree>(graph);
        assert_registered_dependencies::<PostDominatorTree>(graph);
        assert_registered_dependencies::<LoopAnalysis>(graph);
        assert_registered_dependencies::<FunctionLiveness>(graph);
        assert_registered_dependencies::<ConstantPropagation>(graph);
        assert_registered_dependencies::<ScalarEvolution>(graph);
        assert_registered_dependencies::<RangeAnalysis>(graph);
        assert_registered_dependencies::<AliasAnalysis>(graph);
    }

    /// Ensure registry edges match analysis declared dependencies.
    fn assert_registered_dependencies<A: Analysis>(graph: &DependencyGraph) {
        // check each declared dependency edge
        for dependency in A::DEPENDENCIES {
            let dependents = graph.get_dependents(*dependency);
            assert!(
                dependents.contains(&A::ID),
                "missing dependency {dependency} for analysis {}",
                A::ID
            );
        }
    }
}
