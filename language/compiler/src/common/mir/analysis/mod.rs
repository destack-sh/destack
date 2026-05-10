use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use destack_mir as mir;

mod alias;
mod available_expressions;
mod call_graph;
mod call_targets;
mod constant_propagation;
mod control_flow_graph;
mod dataflow;
mod dominator_tree;
mod lifetime;
mod liveness;
mod loop_analysis;
mod memory_ssa;
mod post_dominator_tree;
mod range;
mod reaching_definitions;
mod scalar_evolution;

pub use alias::*;
pub use available_expressions::*;
pub use call_graph::*;
pub use call_targets::*;
pub use constant_propagation::*;
pub use control_flow_graph::*;
pub use dataflow::*;
pub use dominator_tree::*;
pub use lifetime::*;
pub use liveness::*;
pub use loop_analysis::*;
pub use memory_ssa::*;
pub use post_dominator_tree::*;
pub use range::*;
pub use reaching_definitions::*;
pub use scalar_evolution::*;

use crate::optimize::{PackageWorkset, ProgramWorkset};

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
    fn compute(function: &mir::Function, tree: &mir::Tree, analyses: &FunctionAnalyses<'_>)
    -> Self;
}

/// Module-scoped analysis.
pub trait ModuleAnalysis: Analysis {
    /// Compute this analysis for the module.
    fn compute(tree: &mir::Tree, analyses: &ModuleAnalyses<'_>) -> Self;
}

/// Package scoped analysis.
pub trait PackageAnalysis: Analysis {
    /// Compute this analysis for a package workset.
    fn compute(workset: &PackageWorkset, analyses: &PackageAnalyses<'_>) -> Self;
}

/// Program scoped analysis.
pub trait ProgramAnalysis: Analysis {
    /// Compute this analysis for a program workset.
    fn compute(workset: &ProgramWorkset, analyses: &ProgramAnalyses<'_>) -> Self;
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

/// Register an analysis in the dependency graph.
fn register_analysis<A: Analysis>(graph: &mut DependencyGraph) {
    graph.register(A::ID, A::DEPENDENCIES);
}

/// Stores function-scoped analyses with lazy computation and dependency tracking.
pub struct FunctionAnalyses<'a> {
    function: &'a mir::Function,
    tree: &'a mir::Tree,
    options: MirAnalysisOptions,
    cache: RefCell<HashMap<AnalysisId, Arc<dyn Any + Send + Sync>>>,
    dependency_graph: DependencyGraph,
}

impl<'a> FunctionAnalyses<'a> {
    /// Create a new function analyses storage with default options.
    pub fn new(function: &'a mir::Function, tree: &'a mir::Tree) -> Self {
        Self::with_options(function, tree, MirAnalysisOptions::default())
    }

    /// Create a new function analyses storage with the given options.
    pub fn with_options(
        function: &'a mir::Function,
        tree: &'a mir::Tree,
        options: MirAnalysisOptions,
    ) -> Self {
        let mut dependency_graph = DependencyGraph::default();

        Self::register_all(&mut dependency_graph);

        Self {
            function,
            tree,
            options,
            cache: RefCell::new(HashMap::new()),
            dependency_graph,
        }
    }

    /// Register all known function analyses.
    fn register_all(graph: &mut DependencyGraph) {
        register_analysis::<ControlFlowGraph>(graph);
        register_analysis::<DominatorTree>(graph);
        register_analysis::<PostDominatorTree>(graph);
        register_analysis::<LoopAnalysis>(graph);
        register_analysis::<LivenessAnalysis>(graph);
        register_analysis::<ConstantPropagation>(graph);
        register_analysis::<ReachingDefinitions>(graph);
        register_analysis::<AvailableExpressions>(graph);
        register_analysis::<ScalarEvolution>(graph);
        register_analysis::<RangeAnalysis>(graph);
        register_analysis::<AliasAnalysis>(graph);
        register_analysis::<MemorySSA>(graph);
    }

    /// Get the function being analyzed.
    pub fn function(&self) -> &mir::Function {
        self.function
    }

    /// Get the tree.
    pub fn tree(&self) -> &mir::Tree {
        self.tree
    }

    /// Get pipeline options.
    pub fn options(&self) -> &MirAnalysisOptions {
        &self.options
    }

    /// Return the type context for this analysis run.
    pub fn type_context(&self) -> TypeContext {
        self.options.type_context
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
    tree: &'a mir::Tree,
    cache: RefCell<HashMap<AnalysisId, Arc<dyn Any + Send + Sync>>>,
    dependency_graph: DependencyGraph,
}

impl<'a> ModuleAnalyses<'a> {
    /// Create a new module analyses storage.
    pub fn new(tree: &'a mir::Tree) -> Self {
        let mut dependency_graph = DependencyGraph::default();
        Self::register_all(&mut dependency_graph);

        Self {
            tree,
            cache: RefCell::new(HashMap::new()),
            dependency_graph,
        }
    }

    /// Register all known module analyses.
    fn register_all(graph: &mut DependencyGraph) {
        register_analysis::<LifetimeAnalysis>(graph);
        register_analysis::<CallGraph>(graph);
        register_analysis::<CallTargetAnalysis>(graph);
    }

    /// Get the tree.
    pub fn tree(&self) -> &mir::Tree {
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

/// Stores package scoped analyses with lazy computation.
pub struct PackageAnalyses<'a> {
    workset: &'a PackageWorkset,
    cache: RefCell<HashMap<AnalysisId, Arc<dyn Any + Send + Sync>>>,
    dependency_graph: DependencyGraph,
}

impl<'a> PackageAnalyses<'a> {
    /// Create a new package analyses storage.
    pub fn new(workset: &'a PackageWorkset) -> Self {
        // build the dependency graph
        let mut dependency_graph = DependencyGraph::default();
        Self::register_all(&mut dependency_graph);

        // store workset and caches
        Self {
            workset,
            cache: RefCell::new(HashMap::new()),
            dependency_graph,
        }
    }

    /// Register all known package analyses.
    fn register_all(graph: &mut DependencyGraph) {
        register_analysis::<PackageCallGraph>(graph);
    }

    /// Get the workset.
    pub fn workset(&self) -> &PackageWorkset {
        self.workset
    }

    /// Get or compute a package analysis.
    pub fn get<A: PackageAnalysis>(&self) -> Arc<A> {
        let id = A::ID;

        // check cache
        if let Some(cached) = self.cache.borrow().get(&id).cloned() {
            return cached.downcast::<A>().expect("analysis type mismatch");
        }

        // compute
        let result = Arc::new(A::compute(self.workset, self));

        // cache
        self.cache.borrow_mut().insert(id, result.clone());

        result
    }

    /// Check if an analysis is cached.
    pub fn is_cached<A: PackageAnalysis>(&self) -> bool {
        self.cache.borrow().contains_key(&A::ID)
    }

    /// Invalidate a package analysis and all its dependents.
    pub fn invalidate(&self, id: AnalysisId) {
        // remove the cached analysis
        self.cache.borrow_mut().remove(&id);

        // invalidate dependent analyses
        for dependent in self.dependency_graph.get_dependents(id) {
            self.invalidate(*dependent);
        }
    }

    /// Invalidate all cached analyses.
    pub fn invalidate_all(&self) {
        // clear all cached analyses
        self.cache.borrow_mut().clear();
    }
}

impl std::fmt::Debug for PackageAnalyses<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PackageAnalyses")
            .field("cached_count", &self.cache.borrow().len())
            .finish()
    }
}

/// Stores program scoped analyses with lazy computation.
pub struct ProgramAnalyses<'a> {
    workset: &'a ProgramWorkset,
    cache: RefCell<HashMap<AnalysisId, Arc<dyn Any + Send + Sync>>>,
    dependency_graph: DependencyGraph,
}

impl<'a> ProgramAnalyses<'a> {
    /// Create a new program analyses storage.
    pub fn new(workset: &'a ProgramWorkset) -> Self {
        // build the dependency graph
        let mut dependency_graph = DependencyGraph::default();
        Self::register_all(&mut dependency_graph);

        // store workset and caches
        Self {
            workset,
            cache: RefCell::new(HashMap::new()),
            dependency_graph,
        }
    }

    /// Register all known program analyses.
    fn register_all(graph: &mut DependencyGraph) {
        register_analysis::<ProgramCallGraph>(graph);
    }

    /// Get the workset.
    pub fn workset(&self) -> &ProgramWorkset {
        self.workset
    }

    /// Get or compute a program analysis.
    pub fn get<A: ProgramAnalysis>(&self) -> Arc<A> {
        let id = A::ID;

        // check cache
        if let Some(cached) = self.cache.borrow().get(&id).cloned() {
            return cached.downcast::<A>().expect("analysis type mismatch");
        }

        // compute
        let result = Arc::new(A::compute(self.workset, self));

        // cache
        self.cache.borrow_mut().insert(id, result.clone());

        result
    }

    /// Check if an analysis is cached.
    pub fn is_cached<A: ProgramAnalysis>(&self) -> bool {
        self.cache.borrow().contains_key(&A::ID)
    }

    /// Invalidate a program analysis and all its dependents.
    pub fn invalidate(&self, id: AnalysisId) {
        // remove the cached analysis
        self.cache.borrow_mut().remove(&id);

        // invalidate dependent analyses
        for dependent in self.dependency_graph.get_dependents(id) {
            self.invalidate(*dependent);
        }
    }

    /// Invalidate all cached analyses.
    pub fn invalidate_all(&self) {
        // clear all cached analyses
        self.cache.borrow_mut().clear();
    }
}

impl std::fmt::Debug for ProgramAnalyses<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgramAnalyses")
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

#[cfg(test)]
mod tests {
    use destack_mir as mir;

    use super::*;
    use crate::optimize::common::tests::TestProgram;

    /// Ensure the dependency registry matches each analysis declaration.
    #[test]
    fn test_function_analysis_dependency_registry() {
        // build a minimal mir program
        let program = TestProgram::new(
            r#"
function test(): void {
b0:
    return
}"#,
        );

        // locate the function and dependency graph
        let function_id = program.tree.iter_nodes::<mir::Function>().next().unwrap().0;
        let function = program.tree.get(function_id);
        let analyses = FunctionAnalyses::new(function, &program.tree);
        let graph = &analyses.dependency_graph;

        // validate dependency edges for all registered analyses
        assert_registered_dependencies::<ControlFlowGraph>(graph);
        assert_registered_dependencies::<DominatorTree>(graph);
        assert_registered_dependencies::<PostDominatorTree>(graph);
        assert_registered_dependencies::<LoopAnalysis>(graph);
        assert_registered_dependencies::<LivenessAnalysis>(graph);
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
