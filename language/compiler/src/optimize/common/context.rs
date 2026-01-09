use super::analysis::{AnalysisCache, AnalysisPreservation};

/// Options that control optimization behavior.
#[derive(Debug, Clone, Copy)]
pub struct OptimizeOptions {
    /// Whether `&mut T` has noalias semantics (like Rust's strict borrowing).
    pub strict_borrow_mode: bool,
    /// Maximum array elements for SROA to split (larger arrays are left intact).
    pub sroa_max_array_elements: usize,
}

impl Default for OptimizeOptions {
    fn default() -> Self {
        Self {
            strict_borrow_mode: false,
            sroa_max_array_elements: 8,
        }
    }
}

/// Context for running optimization passes.
///
/// Provides access to cached analyses and shared resources like the string pool.
#[derive(Debug)]
pub struct OptimizationContext<'a> {
    /// String pool for looking up identifiers.
    pub strings: &'a destack_base::StringPool,
    /// Cached analyses for the current function.
    pub analyses: AnalysisCache,
    /// Optimization options.
    pub options: OptimizeOptions,
}

impl<'a> OptimizationContext<'a> {
    /// Create a new optimization context with the given options.
    pub fn new(strings: &'a destack_base::StringPool, options: OptimizeOptions) -> Self {
        Self {
            strings,
            analyses: AnalysisCache::new(),
            options,
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
