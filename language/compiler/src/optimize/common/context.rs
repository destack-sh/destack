use std::sync::atomic::{AtomicBool, Ordering};

use destack_source::ModuleId;
use destack_workspace::TargetId;
use parking_lot::Mutex;

use super::analysis::{AnalysisCache, AnalysisPreservation};
use crate::{OptimizeError, OptimizeWarning};

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
/// Provides access to cached analyses, shared resources, and diagnostic accumulation.
pub struct OptimizationContext<'a> {
    /// String pool for looking up identifiers.
    pub strings: &'a destack_base::StringPool,
    /// Cached analyses for the current function.
    pub analyses: AnalysisCache,
    /// Optimization options.
    pub options: OptimizeOptions,

    /// The module being optimized.
    module_id: ModuleId,
    /// The target being optimized.
    target_id: TargetId,

    /// Accumulated errors from verification passes.
    errors: Mutex<Vec<OptimizeError>>,
    /// Accumulated warnings from verification passes.
    warnings: Mutex<Vec<OptimizeWarning>>,
    /// Whether all verification passed with no aliasing violations.
    /// When true in lenient mode, we can still use strict aliasing for optimizations.
    is_strict_safe: AtomicBool,
}

impl std::fmt::Debug for OptimizationContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OptimizationContext")
            .field("strings", &"...")
            .field("analyses", &self.analyses)
            .field("options", &self.options)
            .field("module_id", &self.module_id)
            .field("target_id", &self.target_id)
            .field("errors", &self.errors.lock().len())
            .field("warnings", &self.warnings.lock().len())
            .field(
                "is_strict_safe",
                &self.is_strict_safe.load(Ordering::Relaxed),
            )
            .finish()
    }
}

impl<'a> OptimizationContext<'a> {
    /// Create a new optimization context with the given options.
    pub fn new(
        strings: &'a destack_base::StringPool,
        options: OptimizeOptions,
        module_id: ModuleId,
        target_id: TargetId,
    ) -> Self {
        Self {
            strings,
            analyses: AnalysisCache::new(),
            options,
            module_id,
            target_id,
            errors: Mutex::new(Vec::new()),
            warnings: Mutex::new(Vec::new()),
            is_strict_safe: AtomicBool::new(true),
        }
    }

    /// Get the module id.
    pub fn module_id(&self) -> ModuleId {
        self.module_id
    }

    /// Get the target id.
    pub fn target_id(&self) -> &TargetId {
        &self.target_id
    }

    /// Clear cached analyses (call between functions or when CFG changes).
    pub fn clear_analyses(&self) {
        self.analyses.clear();
    }

    /// Invalidate analyses based on what a pass preserved.
    pub fn invalidate(&self, preserved: &AnalysisPreservation) {
        self.analyses.invalidate(preserved);
    }

    // -------------------------------------------------------------------------
    // Diagnostic accumulation
    // -------------------------------------------------------------------------

    /// Emit an optimization error.
    pub fn emit_error(&self, error: OptimizeError) {
        self.errors.lock().push(error);
    }

    /// Emit an optimization warning.
    pub fn emit_warning(&self, warning: OptimizeWarning) {
        self.warnings.lock().push(warning);
    }

    /// Mark that aliasing violations were found (code is not strict-safe).
    pub fn mark_aliasing_violation(&self) {
        self.is_strict_safe.store(false, Ordering::Relaxed);
    }

    /// Check if code is strict-safe (no aliasing violations found in lenient mode).
    pub fn is_strict_safe(&self) -> bool {
        self.is_strict_safe.load(Ordering::Relaxed)
    }

    /// Check if `&mut T` should have noalias semantics for optimization.
    /// True if strict mode enabled OR lenient mode with no violations.
    pub fn use_strict_aliasing(&self) -> bool {
        self.options.strict_borrow_mode || self.is_strict_safe()
    }

    /// Take all accumulated errors.
    pub fn take_errors(&self) -> Vec<OptimizeError> {
        std::mem::take(&mut *self.errors.lock())
    }

    /// Take all accumulated warnings.
    pub fn take_warnings(&self) -> Vec<OptimizeWarning> {
        std::mem::take(&mut *self.warnings.lock())
    }

    /// Check if any errors were accumulated.
    pub fn has_errors(&self) -> bool {
        !self.errors.lock().is_empty()
    }

    /// Check if any warnings were accumulated.
    pub fn has_warnings(&self) -> bool {
        !self.warnings.lock().is_empty()
    }
}
