use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use crate as mir;

use super::Mutation;

/// Target layout facts for MIR analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetLayout {
    /// Pointer width in bits for pointer sized integers.
    pub pointer_width_bits: u16,
}

impl Default for TargetLayout {
    fn default() -> Self {
        Self {
            pointer_width_bits: usize::BITS as u16,
        }
    }
}

/// Options used by MIR analyses.
#[derive(Debug, Clone, Copy, Default)]
pub struct AnalysisOptions {
    /// Target layout for layout sensitive analyses.
    pub target_layout: TargetLayout,
}

impl AnalysisOptions {
    /// Create MIR analysis options.
    pub fn new(target_layout: TargetLayout) -> Self {
        Self { target_layout }
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
    fn compute(function: &mir::Function, tree: &mir::Tree, analyses: &FunctionAnalyses) -> Self;
}

/// Module-scoped analysis.
pub trait ModuleAnalysis: Analysis {
    /// Compute this analysis for the module.
    fn compute(tree: &mir::Tree, analyses: &ModuleAnalyses) -> Self;
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
            return cached
                .clone()
                .downcast::<A>()
                .expect("analysis type mismatch");
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

/// Caches function-scoped analyses across one function's pass sequence.
///
/// A pipeline holds one cache per function and applies each pass's reported
/// mutation to drop stale entries.
#[derive(Debug, Default)]
pub struct FunctionAnalyses {
    cache: AnalysisCache,
    options: AnalysisOptions,
}

impl FunctionAnalyses {
    /// Create a new function analysis cache with default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new function analysis cache with the given options.
    pub fn with_options(options: AnalysisOptions) -> Self {
        Self {
            cache: AnalysisCache::new(),
            options,
        }
    }

    /// Get the analysis options.
    pub fn options(&self) -> &AnalysisOptions {
        &self.options
    }

    /// Return the target layout for this analysis run.
    pub fn target_layout(&self) -> TargetLayout {
        self.options.target_layout
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

/// Caches module-scoped analyses across one module's pass sequence.
#[derive(Debug, Default)]
pub struct ModuleAnalyses {
    cache: AnalysisCache,
}

impl ModuleAnalyses {
    /// Create a new module analysis cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or compute a module analysis for the given tree.
    pub fn get<A: ModuleAnalysis>(&self, tree: &mir::Tree) -> Arc<A> {
        self.cache.get_or_compute(|| A::compute(tree, self))
    }

    /// Check if an analysis is cached.
    pub fn is_cached<A: ModuleAnalysis>(&self) -> bool {
        self.cache.is_cached::<A>()
    }

    /// Drop every analysis the given mutation invalidates.
    pub fn apply(&self, mutation: Mutation) {
        self.cache.apply(mutation);
    }
}
