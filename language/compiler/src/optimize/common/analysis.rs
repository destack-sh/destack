use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use destack_mir as mir;

/// Known analysis kinds.
///
/// This is a closed enum - only the compiler can define new analysis types.
/// Used for invalidation tracking in AnalysisPreservation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnalysisKind {
    /// Control flow graph (predecessors).
    ControlFlowGraph,
    /// Dominator tree.
    DominatorTree,
    /// Natural loop analysis.
    LoopAnalysis,
    /// Alias analysis (memory aliasing).
    AliasAnalysis,
}

/// A computed analysis over MIR.
///
/// Analyses are lazily computed and cached. They can depend on other analyses
/// by requesting them from the cache during computation.
pub trait Analysis: 'static + Sized + Send + Sync {
    /// The kind of this analysis for invalidation tracking.
    const KIND: AnalysisKind;

    /// Compute this analysis for a function.
    ///
    /// Dependencies on other analyses should be requested via `cache.get::<OtherAnalysis>()`.
    /// The cache handles lazy computation and caching of dependencies.
    fn compute(function: &mir::Function, tree: &mir::NodeTree, cache: &AnalysisCache) -> Arc<Self>;
}

/// Cache for computed function analyses.
///
/// Stores analysis results and handles lazy computation. Uses interior mutability
/// to allow recursive analysis requests during computation.
pub struct AnalysisCache {
    /// Cached analysis results, keyed by AnalysisKind.
    cache: RefCell<HashMap<AnalysisKind, Arc<dyn Any + Send + Sync>>>,
    /// Whether strict borrow mode is enabled (`&mut T` has noalias semantics).
    strict_borrow_mode: bool,
}

impl AnalysisCache {
    /// Create a new empty cache.
    pub fn new() -> Self {
        Self {
            cache: RefCell::new(HashMap::new()),
            strict_borrow_mode: false,
        }
    }

    /// Create a new cache with the specified strict borrow mode.
    pub fn with_strict_borrow_mode(strict_borrow_mode: bool) -> Self {
        Self {
            cache: RefCell::new(HashMap::new()),
            strict_borrow_mode,
        }
    }

    /// Whether strict borrow mode is enabled (`&mut T` has noalias semantics).
    pub fn strict_borrow_mode(&self) -> bool {
        self.strict_borrow_mode
    }

    /// Get or compute an analysis for a function.
    ///
    /// If the analysis is cached, returns the cached result. Otherwise computes
    /// the analysis (which may recursively request other analyses) and caches it.
    pub fn get<A: Analysis>(&self, function: &mir::Function, tree: &mir::NodeTree) -> Arc<A> {
        let kind = A::KIND;

        // check cache
        if let Some(cached) = self.cache.borrow().get(&kind).cloned() {
            return cached.downcast::<A>().expect("analysis type mismatch");
        }

        // compute (may recursively call get for dependencies)
        let result = A::compute(function, tree, self);

        // cache
        self.cache.borrow_mut().insert(kind, result.clone());

        result
    }

    /// Invalidate analyses based on what was preserved by a pass.
    pub fn invalidate(&self, preserved: &AnalysisPreservation) {
        match preserved {
            AnalysisPreservation::All => {
                // all preserved, nothing to do
            }
            AnalysisPreservation::Some(kinds) if kinds.is_empty() => {
                // nothing preserved, clear everything
                self.cache.borrow_mut().clear();
            }
            AnalysisPreservation::Some(kinds) => {
                // selective invalidation
                self.cache
                    .borrow_mut()
                    .retain(|kind, _| kinds.contains(kind));
            }
        }
    }

    /// Clear all cached analyses.
    pub fn clear(&self) {
        self.cache.borrow_mut().clear();
    }

    /// Check if an analysis is cached.
    pub fn is_cached<A: Analysis>(&self) -> bool {
        self.cache.borrow().contains_key(&A::KIND)
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
            .field("cached_count", &self.cache.borrow().len())
            .finish()
    }
}

/// Declares what analyses are still valid after a pass runs.
///
/// Passes return this to tell the optimizer which cached analyses can be kept.
/// This enables efficient caching without recomputing analyses unnecessarily.
#[derive(Debug, Clone)]
pub enum AnalysisPreservation {
    /// All analyses are preserved (pass made no changes).
    All,
    /// Only specific analyses are preserved.
    Some(Vec<AnalysisKind>),
}

impl AnalysisPreservation {
    /// All analyses are preserved (pass made no changes).
    pub fn all() -> Self {
        Self::All
    }

    /// No analyses are preserved (pass may have changed anything).
    pub fn none() -> Self {
        Self::Some(Vec::new())
    }

    /// Check if all analyses are preserved.
    pub fn preserves_all(&self) -> bool {
        matches!(self, Self::All)
    }

    /// Check if a specific analysis kind is preserved.
    pub fn is_preserved(&self, kind: AnalysisKind) -> bool {
        match self {
            Self::All => true,
            Self::Some(preserved) => preserved.contains(&kind),
        }
    }
}

impl Default for AnalysisPreservation {
    fn default() -> Self {
        Self::none()
    }
}
