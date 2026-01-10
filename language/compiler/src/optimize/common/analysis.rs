use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use destack_mir as mir;

use super::pipeline::PipelineOptions;

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
/// Concrete analyses implement either `FunctionAnalysis` or `ModuleAnalysis`.
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
    fn compute(
        function: &mir::Function,
        tree: &mir::NodeTree,
        analyses: &FunctionAnalyses<'_>,
    ) -> Self;
}

/// Module-scoped analysis.
pub trait ModuleAnalysis: Analysis {
    /// Compute this analysis for the module.
    fn compute(tree: &mir::NodeTree, analyses: &ModuleAnalyses<'_>) -> Self;
}

/// Dependency graph for cascading invalidation.
#[derive(Debug, Default)]
struct DependencyGraph {
    /// Maps each analysis to analyses that depend on it.
    dependents: HashMap<AnalysisId, Vec<AnalysisId>>,
}

impl DependencyGraph {
    /// Register an analysis and its dependencies.
    fn register(&mut self, id: AnalysisId, dependencies: &[AnalysisId]) {
        for dep in dependencies {
            self.dependents.entry(*dep).or_default().push(id);
        }
    }

    /// Get all analyses that directly depend on the given analysis.
    fn get_dependents(&self, id: AnalysisId) -> &[AnalysisId] {
        self.dependents
            .get(&id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

/// Stores function-scoped analyses with lazy computation and dependency tracking.
pub struct FunctionAnalyses<'a> {
    function: &'a mir::Function,
    tree: &'a mir::NodeTree,
    options: PipelineOptions,
    cache: RefCell<HashMap<AnalysisId, Arc<dyn Any + Send + Sync>>>,
    dependency_graph: DependencyGraph,
}

impl<'a> FunctionAnalyses<'a> {
    /// Create a new function analyses storage with default options.
    pub fn new(function: &'a mir::Function, tree: &'a mir::NodeTree) -> Self {
        Self::with_options(function, tree, PipelineOptions::default())
    }

    /// Create a new function analyses storage with the given options.
    pub fn with_options(
        function: &'a mir::Function,
        tree: &'a mir::NodeTree,
        options: PipelineOptions,
    ) -> Self {
        let mut dependency_graph = DependencyGraph::default();

        // register known function analyses and their dependencies
        // (this will be populated as analyses are migrated)
        Self::register_known_analyses(&mut dependency_graph);

        Self {
            function,
            tree,
            options,
            cache: RefCell::new(HashMap::new()),
            dependency_graph,
        }
    }

    /// Register all known function analyses.
    fn register_known_analyses(graph: &mut DependencyGraph) {
        // CFG has no dependencies
        graph.register(AnalysisId("cfg"), &[]);

        // dominator tree depends on CFG
        graph.register(AnalysisId("domtree"), &[AnalysisId("cfg")]);

        // post-dominator tree depends on CFG
        graph.register(AnalysisId("postdomtree"), &[AnalysisId("cfg")]);

        // loop analysis depends on dominator tree
        graph.register(AnalysisId("loops"), &[AnalysisId("domtree")]);

        // liveness depends on CFG
        graph.register(AnalysisId("liveness"), &[AnalysisId("cfg")]);

        // ownership depends on CFG
        graph.register(AnalysisId("ownership"), &[AnalysisId("cfg")]);

        // borrow depends on ownership
        graph.register(AnalysisId("borrow"), &[AnalysisId("ownership")]);

        // alias analysis depends on CFG
        graph.register(AnalysisId("alias"), &[AnalysisId("cfg")]);

        // scalar evolution depends on loops
        graph.register(AnalysisId("scev"), &[AnalysisId("loops")]);

        // range analysis depends on scalar evolution
        graph.register(AnalysisId("range"), &[AnalysisId("scev")]);

        // constant propagation depends on CFG
        graph.register(AnalysisId("constprop"), &[AnalysisId("cfg")]);
    }

    /// Get the function being analyzed.
    pub fn function(&self) -> &mir::Function {
        self.function
    }

    /// Get the node tree.
    pub fn tree(&self) -> &mir::NodeTree {
        self.tree
    }

    /// Get pipeline options.
    pub fn options(&self) -> &PipelineOptions {
        &self.options
    }

    /// Get or compute a function analysis.
    pub fn get<A: FunctionAnalysis>(&self) -> Arc<A> {
        let id = A::ID;

        // check cache
        if let Some(cached) = self.cache.borrow().get(&id).cloned() {
            return cached.downcast::<A>().expect("analysis type mismatch");
        }

        // compute (may recursively call get for dependencies)
        let result = Arc::new(A::compute(self.function, self.tree, self));

        // cache
        self.cache.borrow_mut().insert(id, result.clone());

        result
    }

    /// Check if an analysis is cached.
    pub fn is_cached<A: FunctionAnalysis>(&self) -> bool {
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

    /// Invalidate all cached analyses.
    pub fn invalidate_all(&self) {
        self.cache.borrow_mut().clear();
    }

    /// Invalidate based on preservation info.
    pub fn apply_preservation(&self, preserved: &AnalysisPreservation) {
        match preserved {
            AnalysisPreservation::All => {
                // nothing to invalidate
            }
            AnalysisPreservation::Some(ids) if ids.is_empty() => {
                // nothing preserved, clear all
                self.invalidate_all();
            }
            AnalysisPreservation::Some(preserved_ids) => {
                // remove anything not preserved
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
}

impl std::fmt::Debug for FunctionAnalyses<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FunctionAnalyses")
            .field("cached_count", &self.cache.borrow().len())
            .finish()
    }
}

/// Stores module-scoped analyses with lazy computation.
pub struct ModuleAnalyses<'a> {
    tree: &'a mir::NodeTree,
    cache: RefCell<HashMap<AnalysisId, Arc<dyn Any + Send + Sync>>>,
    dependency_graph: DependencyGraph,
}

impl<'a> ModuleAnalyses<'a> {
    /// Create a new module analyses storage.
    pub fn new(tree: &'a mir::NodeTree) -> Self {
        let mut dependency_graph = DependencyGraph::default();
        Self::register_known_analyses(&mut dependency_graph);

        Self {
            tree,
            cache: RefCell::new(HashMap::new()),
            dependency_graph,
        }
    }

    /// Register all known module analyses.
    fn register_known_analyses(graph: &mut DependencyGraph) {
        // lifetime analysis has no dependencies
        graph.register(AnalysisId("lifetime"), &[]);

        // call graph has no dependencies
        graph.register(AnalysisId("callgraph"), &[]);
    }

    /// Get the node tree.
    pub fn tree(&self) -> &mir::NodeTree {
        self.tree
    }

    /// Get or compute a module analysis.
    pub fn get<A: ModuleAnalysis>(&self) -> Arc<A> {
        let id = A::ID;

        // check cache
        if let Some(cached) = self.cache.borrow().get(&id).cloned() {
            return cached.downcast::<A>().expect("analysis type mismatch");
        }

        // compute
        let result = Arc::new(A::compute(self.tree, self));

        // cache
        self.cache.borrow_mut().insert(id, result.clone());

        result
    }

    /// Check if an analysis is cached.
    pub fn is_cached<A: ModuleAnalysis>(&self) -> bool {
        self.cache.borrow().contains_key(&A::ID)
    }

    /// Create function analyses for a specific function.
    pub fn function_analyses(&self, function: &'a mir::Function) -> FunctionAnalyses<'a> {
        FunctionAnalyses::new(function, self.tree)
    }

    /// Invalidate a module analysis and all its dependents.
    pub fn invalidate(&self, id: AnalysisId) {
        self.cache.borrow_mut().remove(&id);

        for dependent in self.dependency_graph.get_dependents(id) {
            self.invalidate(*dependent);
        }
    }

    /// Invalidate all cached analyses.
    pub fn invalidate_all(&self) {
        self.cache.borrow_mut().clear();
    }
}

impl std::fmt::Debug for ModuleAnalyses<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModuleAnalyses")
            .field("cached_count", &self.cache.borrow().len())
            .finish()
    }
}

/// Declares what analyses are still valid after a pass runs (new version).
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
