use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use destack_artifact::{DiagnosticBuilder, ProgramAnalysis};
use destack_core::StringPool;
use destack_mir as mir;
use destack_source::{ModuleId, PackageId, ProfileId, TargetId};
use parking_lot::Mutex;

use crate::optimize::{DiagnosticEmitter, ModuleWorkItem, PackageWorkset, PassMetadata};
use crate::{DiagnosticAnchor, OptimizeError, OptimizeWarning};
use destack_mir::{AnalysisOptions, FunctionAnalyses, HotnessThresholds, TargetLayout};

/// Shared diagnostics state for pipeline contexts.
#[derive(Debug)]
pub struct PipelineDiagnostics {
    /// Accumulated errors from pipeline passes.
    errors: Mutex<Vec<DiagnosticBuilder<OptimizeError>>>,
    /// Accumulated warnings from pipeline passes.
    warnings: Mutex<Vec<DiagnosticBuilder<OptimizeWarning>>>,
    /// Whether type layouts have been validated for this pipeline run.
    type_layouts_validated: AtomicBool,
}

impl PipelineDiagnostics {
    /// Create a new diagnostics state.
    pub fn new() -> Self {
        Self {
            errors: Mutex::new(Vec::new()),
            warnings: Mutex::new(Vec::new()),
            type_layouts_validated: AtomicBool::new(false),
        }
    }

    /// Emit an optimization error.
    pub fn emit_error(&self, error: impl Into<DiagnosticBuilder<OptimizeError>>) {
        self.errors.lock().push(error.into());
    }

    /// Emit an optimization warning.
    pub fn emit_warning(&self, warning: impl Into<DiagnosticBuilder<OptimizeWarning>>) {
        self.warnings.lock().push(warning.into());
    }

    /// Return true when type layouts have been validated.
    pub fn type_layouts_validated(&self) -> bool {
        self.type_layouts_validated.load(Ordering::Relaxed)
    }

    /// Mark that type layouts have been validated.
    pub fn mark_type_layouts_validated(&self) {
        self.type_layouts_validated.store(true, Ordering::Relaxed);
    }

    /// Take all accumulated errors.
    pub fn take_errors(&self) -> Vec<DiagnosticBuilder<OptimizeError>> {
        std::mem::take(&mut *self.errors.lock())
    }

    /// Take all accumulated warnings.
    pub fn take_warnings(&self) -> Vec<DiagnosticBuilder<OptimizeWarning>> {
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

impl Default for PipelineDiagnostics {
    fn default() -> Self {
        Self::new()
    }
}

/// Options for pipeline execution.
#[derive(Debug, Clone)]
pub struct PipelineOptions {
    /// Maximum array elements for SROA to split (larger arrays are left intact).
    pub sroa_max_array_elements: usize,
    /// MIR analysis options for this pipeline run.
    pub analysis: AnalysisOptions,
    /// Loop unroll threshold in instructions.
    pub unroll_threshold: usize,
    /// Inline budget scaling for this optimization level.
    pub inline_budget_scale_percent: u64,
}

impl Default for PipelineOptions {
    fn default() -> Self {
        Self {
            sroa_max_array_elements: 8,
            analysis: AnalysisOptions::default(),
            unroll_threshold: 200,
            inline_budget_scale_percent: 100,
        }
    }
}

impl PipelineOptions {
    /// Return the target layout for this pipeline run.
    pub fn target_layout(&self) -> TargetLayout {
        self.analysis.target_layout
    }

    /// Return profile hotness thresholds for this pipeline run.
    pub fn hotness_thresholds(&self) -> HotnessThresholds {
        self.analysis.hotness
    }

    /// Return the loop unroll threshold for this pipeline run.
    pub fn unroll_threshold(&self) -> usize {
        self.unroll_threshold
    }

    /// Return the inline budget scale percent for this pipeline run.
    pub fn inline_budget_scale_percent(&self) -> u64 {
        self.inline_budget_scale_percent
    }
}

/// Context for pipeline execution.
///
/// Provides access to strings, options, and diagnostic accumulation.
/// Module level analyses are created on demand by passes that need them.
pub struct PipelineContext<'a> {
    /// String pool for identifiers.
    pub strings: &'a StringPool,
    /// Optimization options.
    pub options: PipelineOptions,

    /// The module being optimized.
    module_id: ModuleId,
    /// The semantic profile being optimized.
    profile_id: ProfileId,
    /// The target being optimized.
    target_id: TargetId,
    /// Optional profile guided optimization data.
    profile: Option<Arc<mir::Profile>>,
    /// Whole-program analysis shared across the modules of this profile and target.
    program_analysis: Arc<ProgramAnalysis>,

    /// Shared diagnostics state.
    diagnostics: Arc<PipelineDiagnostics>,
}

impl std::fmt::Debug for PipelineContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipelineContext")
            .field("strings", &"...")
            .field("options", &self.options)
            .field("module_id", &self.module_id)
            .field("profile_id", &self.profile_id)
            .field("target_id", &self.target_id)
            .field("has_profile", &self.profile.is_some())
            .field("errors", &self.diagnostics.errors.lock().len())
            .field("warnings", &self.diagnostics.warnings.lock().len())
            .finish()
    }
}

impl<'a> PipelineContext<'a> {
    /// Create a new pipeline context.
    pub fn new(
        strings: &'a StringPool,
        options: PipelineOptions,
        module_id: ModuleId,
        profile_id: ProfileId,
        target_id: TargetId,
        profile: Option<Arc<mir::Profile>>,
        program_analysis: Arc<ProgramAnalysis>,
    ) -> Self {
        Self::with_diagnostics(
            strings,
            options,
            module_id,
            profile_id,
            target_id,
            profile,
            program_analysis,
            Arc::new(PipelineDiagnostics::default()),
        )
    }

    /// Create a new pipeline context with shared diagnostics.
    pub fn with_diagnostics(
        strings: &'a StringPool,
        options: PipelineOptions,
        module_id: ModuleId,
        profile_id: ProfileId,
        target_id: TargetId,
        profile: Option<Arc<mir::Profile>>,
        program_analysis: Arc<ProgramAnalysis>,
        diagnostics: Arc<PipelineDiagnostics>,
    ) -> Self {
        Self {
            strings,
            options,
            module_id,
            profile_id,
            target_id,
            profile,
            program_analysis,
            diagnostics,
        }
    }

    /// Get the module id.
    pub fn module_id(&self) -> ModuleId {
        self.module_id
    }

    /// Get the profile id.
    pub fn profile_id(&self) -> ProfileId {
        self.profile_id
    }

    /// Get the target id.
    pub fn target_id(&self) -> &TargetId {
        &self.target_id
    }

    /// Get profile data if available.
    pub fn profile(&self) -> Option<&mir::Profile> {
        self.profile.as_deref()
    }

    /// Return the program analysis for this optimization scope.
    pub fn program_analysis(&self) -> &ProgramAnalysis {
        &self.program_analysis
    }

    /// Return true when profile data is available.
    pub fn has_profile(&self) -> bool {
        self.profile.is_some()
    }

    /// Return the loop unroll threshold for this pipeline run.
    pub fn unroll_threshold(&self) -> usize {
        self.options.unroll_threshold()
    }

    /// Return the inline budget scale percent for this pipeline run.
    pub fn inline_budget_scale_percent(&self) -> u64 {
        self.options.inline_budget_scale_percent()
    }

    /// Create a diagnostic anchor for one MIR node.
    pub fn anchor(&self, tree: &mir::Tree, node: mir::LocalNodeIdAny) -> DiagnosticAnchor {
        if let Some(span) = tree.source_span_by_id(node.id) {
            DiagnosticAnchor::Span(span)
        } else {
            DiagnosticAnchor::Module(self.module_id)
        }
    }

    /// Enforce metadata requirements for a function pass.
    pub fn enforce_function_requirements(
        &self,
        metadata: &PassMetadata,
        function_id: mir::LocalNodeId<mir::Function>,
        function: &mir::Function,
        tree: &mir::Tree,
    ) -> bool {
        super::contract::enforce_function_requirements(self, metadata, function_id, function, tree)
    }

    /// Enforce metadata requirements for a module pass.
    pub fn enforce_module_requirements(&self, metadata: &PassMetadata, tree: &mir::Tree) -> bool {
        super::contract::enforce_module_requirements(self, metadata, tree)
    }

    /// Create module level analyses for a tree.
    ///
    /// Create an empty function analysis cache configured for this run.
    ///
    /// The pipeline holds one cache per function across that function's pass
    /// sequence and queries it with the function and tree at each access.
    pub fn new_function_analyses(&self) -> FunctionAnalyses {
        FunctionAnalyses::with_options(self.options.analysis)
    }

    /// Return the target layout for this pipeline run.
    pub fn target_layout(&self) -> TargetLayout {
        self.options.target_layout()
    }

    /// Return profile hotness thresholds for this pipeline run.
    pub fn hotness_thresholds(&self) -> HotnessThresholds {
        self.options.hotness_thresholds()
    }

    /// Emit an optimization error.
    pub fn emit_error(&self, error: impl Into<DiagnosticBuilder<OptimizeError>>) {
        self.diagnostics.emit_error(error);
    }

    /// Emit an optimization warning.
    pub fn emit_warning(&self, warning: impl Into<DiagnosticBuilder<OptimizeWarning>>) {
        self.diagnostics.emit_warning(warning);
    }

    /// Take all accumulated errors.
    pub fn take_errors(&self) -> Vec<DiagnosticBuilder<OptimizeError>> {
        self.diagnostics.take_errors()
    }

    /// Take all accumulated warnings.
    pub fn take_warnings(&self) -> Vec<DiagnosticBuilder<OptimizeWarning>> {
        self.diagnostics.take_warnings()
    }

    /// Check if any errors were accumulated.
    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }

    /// Check if any warnings were accumulated.
    pub fn has_warnings(&self) -> bool {
        self.diagnostics.has_warnings()
    }

    /// Get the shared diagnostics state.
    pub fn diagnostics(&self) -> Arc<PipelineDiagnostics> {
        self.diagnostics.clone()
    }
}

impl DiagnosticEmitter for PipelineContext<'_> {
    fn emit_error(&self, error: impl Into<DiagnosticBuilder<OptimizeError>>) {
        self.diagnostics.emit_error(error);
    }

    fn emit_warning(&self, warning: impl Into<DiagnosticBuilder<OptimizeWarning>>) {
        self.diagnostics.emit_warning(warning);
    }
}

/// Context for package pipeline execution.
#[derive(Debug)]
pub struct PackagePipelineContext {
    /// The package being optimized.
    package_id: PackageId,
    /// The target being optimized.
    target_id: TargetId,
    /// Whole-program analysis shared across the package's modules.
    program_analysis: Arc<ProgramAnalysis>,
    /// Shared diagnostics state.
    diagnostics: Arc<PipelineDiagnostics>,
}

impl PackagePipelineContext {
    /// Create a new package pipeline context.
    pub fn new(
        package_id: PackageId,
        target_id: TargetId,
        program_analysis: Arc<ProgramAnalysis>,
    ) -> Self {
        Self::with_diagnostics(
            package_id,
            target_id,
            program_analysis,
            Arc::new(PipelineDiagnostics::default()),
        )
    }

    /// Create a new package pipeline context with shared diagnostics.
    pub fn with_diagnostics(
        package_id: PackageId,
        target_id: TargetId,
        program_analysis: Arc<ProgramAnalysis>,
        diagnostics: Arc<PipelineDiagnostics>,
    ) -> Self {
        Self {
            package_id,
            target_id,
            program_analysis,
            diagnostics,
        }
    }

    /// Get the package id.
    pub fn package_id(&self) -> PackageId {
        self.package_id
    }

    /// Get the target id.
    pub fn target_id(&self) -> &TargetId {
        &self.target_id
    }

    /// Create a module context for a work item.
    pub fn with_module_context<T>(
        &self,
        module: &ModuleWorkItem,
        f: impl for<'a> FnOnce(&mut PipelineContext<'a>) -> T,
    ) -> T {
        // capture module profile
        let profile = module.clone_profile();

        // build the module context
        module.with_strings(|strings| {
            let mut context = PipelineContext::with_diagnostics(
                strings,
                module.options().clone(),
                module.module_id(),
                module.profile_id(),
                *module.target_id(),
                profile,
                self.program_analysis.clone(),
                self.diagnostics.clone(),
            );

            f(&mut context)
        })
    }

    /// Emit an optimization error.
    pub fn emit_error(&self, error: impl Into<DiagnosticBuilder<OptimizeError>>) {
        self.diagnostics.emit_error(error);
    }

    /// Emit an optimization warning.
    pub fn emit_warning(&self, warning: impl Into<DiagnosticBuilder<OptimizeWarning>>) {
        self.diagnostics.emit_warning(warning);
    }

    /// Take all accumulated errors.
    pub fn take_errors(&self) -> Vec<DiagnosticBuilder<OptimizeError>> {
        self.diagnostics.take_errors()
    }

    /// Take all accumulated warnings.
    pub fn take_warnings(&self) -> Vec<DiagnosticBuilder<OptimizeWarning>> {
        self.diagnostics.take_warnings()
    }

    /// Check if any errors were accumulated.
    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }

    /// Check if any warnings were accumulated.
    pub fn has_warnings(&self) -> bool {
        self.diagnostics.has_warnings()
    }

    /// Get the shared diagnostics state.
    pub fn diagnostics(&self) -> Arc<PipelineDiagnostics> {
        self.diagnostics.clone()
    }
}

impl DiagnosticEmitter for PackagePipelineContext {
    fn emit_error(&self, error: impl Into<DiagnosticBuilder<OptimizeError>>) {
        self.diagnostics.emit_error(error);
    }

    fn emit_warning(&self, warning: impl Into<DiagnosticBuilder<OptimizeWarning>>) {
        self.diagnostics.emit_warning(warning);
    }
}

/// Context for program pipeline execution.
#[derive(Debug)]
pub struct ProgramPipelineContext {
    /// The target name being optimized.
    target_name: String,
    /// Whole-program analysis shared across the program's packages.
    program_analysis: Arc<ProgramAnalysis>,
    /// Shared diagnostics state.
    diagnostics: Arc<PipelineDiagnostics>,
}

impl ProgramPipelineContext {
    /// Create a new program pipeline context.
    pub fn new(target_name: String, program_analysis: Arc<ProgramAnalysis>) -> Self {
        Self::with_diagnostics(
            target_name,
            program_analysis,
            Arc::new(PipelineDiagnostics::default()),
        )
    }

    /// Create a new program pipeline context with shared diagnostics.
    pub fn with_diagnostics(
        target_name: String,
        program_analysis: Arc<ProgramAnalysis>,
        diagnostics: Arc<PipelineDiagnostics>,
    ) -> Self {
        Self {
            target_name,
            program_analysis,
            diagnostics,
        }
    }

    /// Get the target name.
    pub fn target_name(&self) -> &str {
        &self.target_name
    }

    /// Create a package context for a workset.
    pub fn with_package_context<T>(
        &self,
        package: &mut PackageWorkset,
        f: impl FnOnce(&mut PackageWorkset, &mut PackagePipelineContext) -> T,
    ) -> T {
        // build the package context
        let mut context = PackagePipelineContext::with_diagnostics(
            package.package_id(),
            *package.target_id(),
            self.program_analysis.clone(),
            self.diagnostics.clone(),
        );

        f(package, &mut context)
    }

    /// Emit an optimization error.
    pub fn emit_error(&self, error: impl Into<DiagnosticBuilder<OptimizeError>>) {
        self.diagnostics.emit_error(error);
    }

    /// Emit an optimization warning.
    pub fn emit_warning(&self, warning: impl Into<DiagnosticBuilder<OptimizeWarning>>) {
        self.diagnostics.emit_warning(warning);
    }

    /// Take all accumulated errors.
    pub fn take_errors(&self) -> Vec<DiagnosticBuilder<OptimizeError>> {
        self.diagnostics.take_errors()
    }

    /// Take all accumulated warnings.
    pub fn take_warnings(&self) -> Vec<DiagnosticBuilder<OptimizeWarning>> {
        self.diagnostics.take_warnings()
    }

    /// Check if any errors were accumulated.
    pub fn has_errors(&self) -> bool {
        self.diagnostics.has_errors()
    }

    /// Check if any warnings were accumulated.
    pub fn has_warnings(&self) -> bool {
        self.diagnostics.has_warnings()
    }

    /// Get the shared diagnostics state.
    pub fn diagnostics(&self) -> Arc<PipelineDiagnostics> {
        self.diagnostics.clone()
    }
}

impl DiagnosticEmitter for ProgramPipelineContext {
    fn emit_error(&self, error: impl Into<DiagnosticBuilder<OptimizeError>>) {
        self.diagnostics.emit_error(error);
    }

    fn emit_warning(&self, warning: impl Into<DiagnosticBuilder<OptimizeWarning>>) {
        self.diagnostics.emit_warning(warning);
    }
}
