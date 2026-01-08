use super::analysis::{AnalysisCache, AnalysisPreservation};

/// Context for running optimization passes.
///
/// Provides access to cached analyses and shared resources like the string pool.
#[derive(Debug)]
pub struct OptimizationContext<'a> {
    /// String pool for looking up identifiers.
    pub strings: &'a destack_base::StringPool,
    /// Cached analyses for the current function.
    pub analyses: AnalysisCache,
}

impl<'a> OptimizationContext<'a> {
    /// Create a new optimization context.
    pub fn new(strings: &'a destack_base::StringPool) -> Self {
        Self {
            strings,
            analyses: AnalysisCache::new(),
        }
    }

    /// Create a new optimization context with strict borrow mode.
    pub fn with_strict_borrow_mode(
        strings: &'a destack_base::StringPool,
        strict_borrow_mode: bool,
    ) -> Self {
        Self {
            strings,
            analyses: AnalysisCache::with_strict_borrow_mode(strict_borrow_mode),
        }
    }

    /// Clear cached analyses (call between functions or when CFG changes).
    pub fn clear_analyses(&self) {
        self.analyses.clear();
    }

    /// Invalidate analyses based on what a pass preserved.
    pub fn invalidate(&self, preserved: &AnalysisPreservation) {
        self.analyses.invalidate(preserved);
    }
}
