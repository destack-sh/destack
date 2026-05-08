use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use destack_artifact::{ArtifactVersion, DiagnosticBuilder};
use destack_core::StringPool;
use destack_mir as mir;
use destack_source::{ModuleId, PackageId, ProfileId, TargetId};
use destack_workspace::FloatMathPolicy;
use parking_lot::Mutex;

use crate::optimize::{
    CallsiteHotnessPolicy, DiagnosticEmitter, FunctionAnalyses, ModuleAnalyses, ModuleWorkItem,
    PackageAnalyses, PackageWorkset, PassMetadata, ProgramAnalyses, ProgramWorkset,
};
use crate::{DiagnosticAnchor, OptimizeError, OptimizeWarning};

/// Shared diagnostics state for pipeline contexts.
#[derive(Debug)]
pub struct PipelineDiagnostics {
    /// Accumulated errors from verification passes.
    errors: Mutex<Vec<DiagnosticBuilder<OptimizeError>>>,
    /// Accumulated warnings from verification passes.
    warnings: Mutex<Vec<DiagnosticBuilder<OptimizeWarning>>>,
    /// Whether all verification passed with no aliasing violations.
    is_strict_safe: AtomicBool,
    /// Whether type layouts have been validated for this pipeline run.
    type_layouts_validated: AtomicBool,
}

impl PipelineDiagnostics {
    /// Create a new diagnostics state.
    pub fn new() -> Self {
        Self {
            errors: Mutex::new(Vec::new()),
            warnings: Mutex::new(Vec::new()),
            is_strict_safe: AtomicBool::new(true),
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

    /// Mark that aliasing violations were found.
    pub fn mark_aliasing_violation(&self) {
        self.is_strict_safe.store(false, Ordering::Relaxed);
    }

    /// Check if code is strict safe.
    pub fn is_strict_safe(&self) -> bool {
        self.is_strict_safe.load(Ordering::Relaxed)
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

/// Type related context for optimization decisions.
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

/// Options for pipeline execution.
#[derive(Debug, Clone)]
pub struct PipelineOptions {
    /// Enable strict borrow checking mode.
    pub strict_borrow_mode: bool,
    /// Maximum array elements for SROA to split (larger arrays are left intact).
    pub sroa_max_array_elements: usize,
    /// Floating point math optimization policy.
    pub float_math: FloatMathPolicy,
    /// Type context for layout sensitive optimizations.
    pub type_context: TypeContext,
    /// Loop unroll threshold in instructions.
    pub unroll_threshold: usize,
    /// Callsite hotness thresholds for inlining decisions.
    pub inline_hotness_policy: CallsiteHotnessPolicy,
    /// Callsite hotness thresholds for argument specialization.
    pub specialize_hotness_policy: CallsiteHotnessPolicy,
    /// Inline budget scaling for this optimization level.
    pub inline_budget_scale_percent: u64,
    /// Enforce optimizable MIR metadata requirements.
    pub require_optimized_metadata: bool,
}

impl Default for PipelineOptions {
    fn default() -> Self {
        Self {
            strict_borrow_mode: false,
            sroa_max_array_elements: 8,
            float_math: FloatMathPolicy::Strict,
            type_context: TypeContext::default(),
            unroll_threshold: 200,
            inline_hotness_policy: CallsiteHotnessPolicy::inline_default(),
            specialize_hotness_policy: CallsiteHotnessPolicy::specialize_default(),
            inline_budget_scale_percent: 100,
            require_optimized_metadata: false,
        }
    }
}

impl PipelineOptions {
    /// Return the type context for this pipeline run.
    pub fn type_context(&self) -> TypeContext {
        self.type_context
    }

    /// Return the loop unroll threshold for this pipeline run.
    pub fn unroll_threshold(&self) -> usize {
        self.unroll_threshold
    }

    /// Return the callsite policy for inline decisions.
    pub fn inline_hotness_policy(&self) -> &CallsiteHotnessPolicy {
        &self.inline_hotness_policy
    }

    /// Return the callsite policy for specialization decisions.
    pub fn specialize_hotness_policy(&self) -> &CallsiteHotnessPolicy {
        &self.specialize_hotness_policy
    }

    /// Return the inline budget scale percent for this pipeline run.
    pub fn inline_budget_scale_percent(&self) -> u64 {
        self.inline_budget_scale_percent
    }

    /// Return true when optimizable MIR metadata is required.
    pub fn require_optimized_metadata(&self) -> bool {
        self.require_optimized_metadata
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
    /// The MIR artifact being optimized.
    source_mir: Option<ArtifactVersion>,
    /// Optional profile guided optimization data.
    profile: Option<Arc<mir::ProfileTable>>,

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
            .field("has_source_mir", &self.source_mir.is_some())
            .field("has_profile", &self.profile.is_some())
            .field("errors", &self.diagnostics.errors.lock().len())
            .field("warnings", &self.diagnostics.warnings.lock().len())
            .field("is_strict_safe", &self.diagnostics.is_strict_safe())
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
        profile: Option<Arc<mir::ProfileTable>>,
    ) -> Self {
        Self::with_diagnostics(
            strings,
            options,
            module_id,
            profile_id,
            target_id,
            None,
            profile,
            Arc::new(PipelineDiagnostics::default()),
        )
    }

    /// Create a new pipeline context from one MIR artifact.
    pub fn with_source_mir(
        strings: &'a StringPool,
        options: PipelineOptions,
        module_id: ModuleId,
        profile_id: ProfileId,
        target_id: TargetId,
        source_mir: ArtifactVersion,
        profile: Option<Arc<mir::ProfileTable>>,
    ) -> Self {
        Self::with_diagnostics(
            strings,
            options,
            module_id,
            profile_id,
            target_id,
            Some(source_mir),
            profile,
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
        source_mir: Option<ArtifactVersion>,
        profile: Option<Arc<mir::ProfileTable>>,
        diagnostics: Arc<PipelineDiagnostics>,
    ) -> Self {
        Self {
            strings,
            options,
            module_id,
            profile_id,
            target_id,
            source_mir,
            profile,
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

    /// Return the MIR artifact being optimized.
    pub fn source_mir(&self) -> ArtifactVersion {
        let Some(source_mir) = self.source_mir else {
            panic!("pipeline pass requires a source MIR artifact");
        };

        source_mir
    }

    /// Get profile data if available.
    pub fn profile(&self) -> Option<&mir::ProfileTable> {
        self.profile.as_deref()
    }

    /// Return true when profile data is available.
    pub fn has_profile(&self) -> bool {
        self.profile.is_some()
    }

    /// Return the loop unroll threshold for this pipeline run.
    pub fn unroll_threshold(&self) -> usize {
        self.options.unroll_threshold()
    }

    /// Return the callsite policy for inline decisions.
    pub fn inline_hotness_policy(&self) -> &CallsiteHotnessPolicy {
        self.options.inline_hotness_policy()
    }

    /// Return the callsite policy for specialization decisions.
    pub fn specialize_hotness_policy(&self) -> &CallsiteHotnessPolicy {
        self.options.specialize_hotness_policy()
    }

    /// Return the inline budget scale percent for this pipeline run.
    pub fn inline_budget_scale_percent(&self) -> u64 {
        self.options.inline_budget_scale_percent()
    }

    /// Return true when optimizable MIR metadata is required.
    pub fn require_optimized_metadata(&self) -> bool {
        self.options.require_optimized_metadata()
    }

    /// Create a diagnostic anchor for one MIR node.
    pub fn anchor(&self, tree: &mir::Tree, node: mir::LocalNodeIdAny) -> DiagnosticAnchor {
        let span = tree
            .get_span_by_id(node.id)
            .expect("optimizer diagnostic node is missing a source span");

        DiagnosticAnchor::Span(span)
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
    /// Module analyses are created on demand since the context doesn't hold
    /// a reference to the tree (to allow mutation during pipeline execution).
    pub fn module_analyses<'b>(&self, tree: &'b mir::Tree) -> ModuleAnalyses<'b> {
        ModuleAnalyses::new(tree)
    }

    /// Create function analyses for a specific function.
    pub fn function_analyses<'b>(
        &self,
        function: &'b mir::Function,
        tree: &'b mir::Tree,
    ) -> FunctionAnalyses<'b> {
        FunctionAnalyses::with_options(function, tree, self.options.clone())
    }

    /// Return the type context for this pipeline run.
    pub fn type_context(&self) -> TypeContext {
        self.options.type_context
    }

    /// Emit an optimization error.
    pub fn emit_error(&self, error: impl Into<DiagnosticBuilder<OptimizeError>>) {
        self.diagnostics.emit_error(error);
    }

    /// Emit an optimization warning.
    pub fn emit_warning(&self, warning: impl Into<DiagnosticBuilder<OptimizeWarning>>) {
        self.diagnostics.emit_warning(warning);
    }

    /// Mark that aliasing violations were found (code is not strict safe).
    pub fn mark_aliasing_violation(&self) {
        self.diagnostics.mark_aliasing_violation();
    }

    /// Check if code is strict safe (no aliasing violations found in lenient mode).
    pub fn is_strict_safe(&self) -> bool {
        self.diagnostics.is_strict_safe()
    }

    /// Check if `&mut T` should have noalias semantics for optimization.
    /// True if strict mode enabled OR lenient mode with no violations.
    pub fn use_strict_aliasing(&self) -> bool {
        self.options.strict_borrow_mode || self.is_strict_safe()
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

    fn mark_aliasing_violation(&self) {
        self.diagnostics.mark_aliasing_violation();
    }
}

/// Context for package pipeline execution.
#[derive(Debug)]
pub struct PackagePipelineContext {
    /// The package being optimized.
    package_id: PackageId,
    /// The target being optimized.
    target_id: TargetId,
    /// Shared diagnostics state.
    diagnostics: Arc<PipelineDiagnostics>,
}

impl PackagePipelineContext {
    /// Create a new package pipeline context.
    pub fn new(package_id: PackageId, target_id: TargetId) -> Self {
        Self::with_diagnostics(
            package_id,
            target_id,
            Arc::new(PipelineDiagnostics::default()),
        )
    }

    /// Create a new package pipeline context with shared diagnostics.
    pub fn with_diagnostics(
        package_id: PackageId,
        target_id: TargetId,
        diagnostics: Arc<PipelineDiagnostics>,
    ) -> Self {
        Self {
            package_id,
            target_id,
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
        // capture module strings and profile
        let strings = module.clone_strings();
        let profile = module.clone_profile();

        // build the module context
        let mut context = PipelineContext::with_diagnostics(
            &strings,
            module.options().clone(),
            module.module_id(),
            module.profile_id(),
            *module.target_id(),
            None,
            profile,
            self.diagnostics.clone(),
        );

        f(&mut context)
    }

    /// Emit an optimization error.
    pub fn emit_error(&self, error: impl Into<DiagnosticBuilder<OptimizeError>>) {
        self.diagnostics.emit_error(error);
    }

    /// Emit an optimization warning.
    pub fn emit_warning(&self, warning: impl Into<DiagnosticBuilder<OptimizeWarning>>) {
        self.diagnostics.emit_warning(warning);
    }

    /// Mark that aliasing violations were found.
    pub fn mark_aliasing_violation(&self) {
        self.diagnostics.mark_aliasing_violation();
    }

    /// Check if code is strict safe.
    pub fn is_strict_safe(&self) -> bool {
        self.diagnostics.is_strict_safe()
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

    /// Create package analyses for a workset.
    pub fn package_analyses<'a>(&self, workset: &'a PackageWorkset) -> PackageAnalyses<'a> {
        PackageAnalyses::new(workset)
    }
}

impl DiagnosticEmitter for PackagePipelineContext {
    fn emit_error(&self, error: impl Into<DiagnosticBuilder<OptimizeError>>) {
        self.diagnostics.emit_error(error);
    }

    fn emit_warning(&self, warning: impl Into<DiagnosticBuilder<OptimizeWarning>>) {
        self.diagnostics.emit_warning(warning);
    }

    fn mark_aliasing_violation(&self) {
        self.diagnostics.mark_aliasing_violation();
    }
}

/// Context for program pipeline execution.
#[derive(Debug)]
pub struct ProgramPipelineContext {
    /// The target name being optimized.
    target_name: String,
    /// Shared diagnostics state.
    diagnostics: Arc<PipelineDiagnostics>,
}

impl ProgramPipelineContext {
    /// Create a new program pipeline context.
    pub fn new(target_name: String) -> Self {
        Self::with_diagnostics(target_name, Arc::new(PipelineDiagnostics::default()))
    }

    /// Create a new program pipeline context with shared diagnostics.
    pub fn with_diagnostics(target_name: String, diagnostics: Arc<PipelineDiagnostics>) -> Self {
        Self {
            target_name,
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

    /// Mark that aliasing violations were found.
    pub fn mark_aliasing_violation(&self) {
        self.diagnostics.mark_aliasing_violation();
    }

    /// Check if code is strict safe.
    pub fn is_strict_safe(&self) -> bool {
        self.diagnostics.is_strict_safe()
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

    /// Create program analyses for a workset.
    pub fn program_analyses<'a>(&self, workset: &'a ProgramWorkset) -> ProgramAnalyses<'a> {
        ProgramAnalyses::new(workset)
    }
}

impl DiagnosticEmitter for ProgramPipelineContext {
    fn emit_error(&self, error: impl Into<DiagnosticBuilder<OptimizeError>>) {
        self.diagnostics.emit_error(error);
    }

    fn emit_warning(&self, warning: impl Into<DiagnosticBuilder<OptimizeWarning>>) {
        self.diagnostics.emit_warning(warning);
    }

    fn mark_aliasing_violation(&self) {
        self.diagnostics.mark_aliasing_violation();
    }
}
