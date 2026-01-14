use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use destack_base::StringPool;
use destack_mir as mir;
use destack_source::ModuleId;
use destack_workspace::{FloatMathPolicy, TargetId};
use parking_lot::Mutex;

use super::analysis::{AnalysisPreservation, FunctionAnalyses, ModuleAnalyses};
use super::context::DiagnosticEmitter;
use super::pass::OptimizationLevel;
use crate::optimize::passes::{
    BorrowCheck, BoundsCheckEliminate, CodeHoisting, ConstantFold, CopyPropagate,
    CorrelatedValueProp, DeadCodeEliminate, DeadStoreEliminate, DropInsert, GlobalValueNumbering,
    GuardEliminate, IfConvert, InductionVariableSimplify, InstructionCombine, Licm,
    LoadStoreForward, LocalCse, LoopBoundsCheckEliminate, LoopDelete, LoopIdiomRecognize, LoopPeel,
    LoopRotate, LoopSimplify, LoopStrengthReduce, LoopUnroll, LoopUnswitch, LoopVersioning,
    Mem2Reg, MemCse, MoveCheck, Narrow, PartialRedundancyElim, Reassociate, SimplifyCfg, Sink,
    SparseConditionalConstantPropagation, Sroa, StackCheck, TailCallElim, ValueRangePropagation,
};
use crate::{OptimizeError, OptimizeWarning};

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
    /// The target being optimized.
    target_id: TargetId,
    /// Optional profile guided optimization data.
    profile: Option<Arc<mir::ProfileTable>>,

    /// Accumulated errors from verification passes.
    errors: Mutex<Vec<OptimizeError>>,
    /// Accumulated warnings from verification passes.
    warnings: Mutex<Vec<OptimizeWarning>>,
    /// Whether all verification passed with no aliasing violations.
    /// NOTE #Architecture: is PipelineContext.is_strict_safe too strict/narrow?
    is_strict_safe: AtomicBool,
}

impl std::fmt::Debug for PipelineContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipelineContext")
            .field("strings", &"...")
            .field("options", &self.options)
            .field("module_id", &self.module_id)
            .field("target_id", &self.target_id)
            .field("has_profile", &self.profile.is_some())
            .field("errors", &self.errors.lock().len())
            .field("warnings", &self.warnings.lock().len())
            .field(
                "is_strict_safe",
                &self.is_strict_safe.load(Ordering::Relaxed),
            )
            .finish()
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
}

impl Default for PipelineOptions {
    fn default() -> Self {
        Self {
            strict_borrow_mode: false,
            sroa_max_array_elements: 8,
            float_math: FloatMathPolicy::Strict,
        }
    }
}

impl<'a> PipelineContext<'a> {
    /// Create a new pipeline context.
    pub fn new(
        strings: &'a StringPool,
        options: PipelineOptions,
        module_id: ModuleId,
        target_id: TargetId,
        profile: Option<Arc<mir::ProfileTable>>,
    ) -> Self {
        Self {
            strings,
            options,
            module_id,
            target_id,
            profile,
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

    /// Get profile data if available.
    pub fn profile(&self) -> Option<&mir::ProfileTable> {
        self.profile.as_deref()
    }

    /// Return true when profile data is available.
    pub fn has_profile(&self) -> bool {
        self.profile.is_some()
    }

    /// Create module level analyses for a tree.
    ///
    /// Module analyses are created on demand since the context doesn't hold
    /// a reference to the tree (to allow mutation during pipeline execution).
    pub fn module_analyses<'b>(&self, tree: &'b mir::NodeTree) -> ModuleAnalyses<'b> {
        ModuleAnalyses::new(tree)
    }

    /// Create function analyses for a specific function.
    pub fn function_analyses<'b>(
        &self,
        function: &'b mir::Function,
        tree: &'b mir::NodeTree,
    ) -> FunctionAnalyses<'b> {
        FunctionAnalyses::with_options(function, tree, self.options.clone())
    }

    /// Emit an optimization error.
    pub fn emit_error(&self, error: OptimizeError) {
        self.errors.lock().push(error);
    }

    /// Emit an optimization warning.
    pub fn emit_warning(&self, warning: OptimizeWarning) {
        self.warnings.lock().push(warning);
    }

    /// Mark that aliasing violations were found (code is not strict safe).
    pub fn mark_aliasing_violation(&self) {
        self.is_strict_safe.store(false, Ordering::Relaxed);
    }

    /// Check if code is strict safe (no aliasing violations found in lenient mode).
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

impl DiagnosticEmitter for PipelineContext<'_> {
    fn emit_error(&self, error: OptimizeError) {
        self.errors.lock().push(error);
    }

    fn emit_warning(&self, warning: OptimizeWarning) {
        self.warnings.lock().push(warning);
    }

    fn mark_aliasing_violation(&self) {
        self.is_strict_safe.store(false, Ordering::Relaxed);
    }
}

/// A composable pipeline element.
///
/// Pipelines can be nested and combined to create complex optimization strategies.
pub trait Pipeline: Send + Sync {
    /// Run the pipeline on a module.
    ///
    /// Returns `true` if any changes were made.
    fn run(&self, tree: &mut mir::NodeTree, ctx: &mut PipelineContext<'_>) -> bool;

    /// Get the name of this pipeline.
    fn name(&self) -> &'static str;
}

/// Trait for passes that operate on individual functions.
///
/// Returns AnalysisPreservation to indicate what analyses are still valid.
pub trait FunctionPass: Send + Sync {
    /// Run the pass on a function.
    ///
    /// The context provides access to strings, options, and diagnostic emission.
    /// Passes that need analyses can create `FunctionAnalyses::new(func, tree)`.
    fn run(
        &self,
        func: &mut mir::Function,
        tree: &mut mir::NodeTree,
        ctx: &PipelineContext<'_>,
    ) -> AnalysisPreservation;

    /// Get the pass name.
    fn name(&self) -> &'static str;

    /// Get the pass ID (for deduplication).
    fn id(&self) -> &'static str {
        self.name()
    }
}

/// Trait for passes that operate on entire modules.
///
/// Returns AnalysisPreservation to indicate what analyses are still valid.
pub trait ModulePass: Send + Sync {
    /// Run the pass on a module.
    ///
    /// The context provides access to strings, options, analyses, and diagnostics.
    fn run(&self, tree: &mut mir::NodeTree, ctx: &PipelineContext<'_>) -> AnalysisPreservation;

    /// Get the pass name.
    fn name(&self) -> &'static str;

    /// Get the pass ID (for deduplication).
    fn id(&self) -> &'static str {
        self.name()
    }
}

/// Pipeline that runs function passes on each function in the module.
pub struct FunctionPipeline {
    passes: Vec<Box<dyn FunctionPass>>,
}

impl fmt::Debug for FunctionPipeline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunctionPipeline")
            .field("passes", &self.passes.len())
            .finish()
    }
}

impl FunctionPipeline {
    /// Create a new function pipeline.
    pub fn new(passes: Vec<Box<dyn FunctionPass>>) -> Self {
        Self { passes }
    }

    /// Create an empty pipeline.
    pub fn empty() -> Self {
        Self { passes: Vec::new() }
    }

    /// Add a pass to the pipeline.
    pub fn add<P: FunctionPass + 'static>(&mut self, pass: P) {
        self.passes.push(Box::new(pass));
    }

    /// Get the number of passes.
    pub fn len(&self) -> usize {
        self.passes.len()
    }

    /// Check if the pipeline is empty.
    pub fn is_empty(&self) -> bool {
        self.passes.is_empty()
    }
}

impl Pipeline for FunctionPipeline {
    fn run(&self, tree: &mut mir::NodeTree, ctx: &mut PipelineContext<'_>) -> bool {
        let mut any_changed = false;

        // collect function ids
        let function_ids: Vec<_> = tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .collect();

        for function_id in function_ids {
            let mut function = tree.get(function_id).clone();

            // skip imported functions (no body)
            if function.entry.is_none() {
                continue;
            }

            for pass in &self.passes {
                // recompute next_value_id so passes can allocate fresh values
                function.recompute_next_value_id(tree);
                let preserved = pass.run(&mut function, tree, ctx);
                if !preserved.preserves_all() {
                    any_changed = true;
                }
            }

            // write function back
            *tree.get_mut(function_id) = function;
        }

        any_changed
    }

    fn name(&self) -> &'static str {
        "FunctionPipeline"
    }
}

/// Pipeline that runs module passes in sequence.
pub struct ModulePipeline {
    passes: Vec<Box<dyn ModulePass>>,
}

impl fmt::Debug for ModulePipeline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModulePipeline")
            .field("passes", &self.passes.len())
            .finish()
    }
}

impl ModulePipeline {
    /// Create a new module pipeline.
    pub fn new(passes: Vec<Box<dyn ModulePass>>) -> Self {
        Self { passes }
    }

    /// Create an empty pipeline.
    pub fn empty() -> Self {
        Self { passes: Vec::new() }
    }

    /// Add a pass to the pipeline.
    pub fn add<P: ModulePass + 'static>(&mut self, pass: P) {
        self.passes.push(Box::new(pass));
    }
}

impl Pipeline for ModulePipeline {
    fn run(&self, tree: &mut mir::NodeTree, ctx: &mut PipelineContext<'_>) -> bool {
        let mut any_changed = false;

        for pass in &self.passes {
            let preserved = pass.run(tree, ctx);

            if !preserved.preserves_all() {
                any_changed = true;
            }
        }

        any_changed
    }

    fn name(&self) -> &'static str {
        "ModulePipeline"
    }
}

/// Adaptor that wraps a function pipeline to run as a module level pipeline.
#[derive(Debug)]
pub struct FunctionToModuleAdaptor {
    inner: FunctionPipeline,
}

impl FunctionToModuleAdaptor {
    /// Create a new adaptor.
    pub fn new(inner: FunctionPipeline) -> Self {
        Self { inner }
    }
}

impl Pipeline for FunctionToModuleAdaptor {
    fn run(&self, tree: &mut mir::NodeTree, ctx: &mut PipelineContext<'_>) -> bool {
        self.inner.run(tree, ctx)
    }

    fn name(&self) -> &'static str {
        "FunctionToModule"
    }
}

/// Pipeline that repeats an inner pipeline until no changes or max iterations.
pub struct RepeatedPipeline<P: Pipeline> {
    inner: P,
    max_iterations: usize,
}

impl<P: Pipeline> std::fmt::Debug for RepeatedPipeline<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RepeatedPipeline")
            .field("inner", &self.inner.name())
            .field("max_iterations", &self.max_iterations)
            .finish()
    }
}

impl<P: Pipeline> RepeatedPipeline<P> {
    /// Create a new repeated pipeline.
    pub fn new(inner: P, max_iterations: usize) -> Self {
        Self {
            inner,
            max_iterations,
        }
    }
}

impl<P: Pipeline> Pipeline for RepeatedPipeline<P> {
    fn run(&self, tree: &mut mir::NodeTree, ctx: &mut PipelineContext<'_>) -> bool {
        let mut any_changed = false;

        for _ in 0..self.max_iterations {
            let changed = self.inner.run(tree, ctx);
            any_changed |= changed;

            // stop if no changes (fixed point reached)
            if !changed {
                break;
            }
        }

        any_changed
    }

    fn name(&self) -> &'static str {
        "RepeatedPipeline"
    }
}

/// Composite pipeline that runs multiple pipelines in sequence.
pub struct CompositePipeline {
    pipelines: Vec<Box<dyn Pipeline>>,
}

impl std::fmt::Debug for CompositePipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositePipeline")
            .field(
                "pipelines",
                &self.pipelines.iter().map(|p| p.name()).collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl CompositePipeline {
    /// Create a new composite pipeline.
    pub fn new(pipelines: Vec<Box<dyn Pipeline>>) -> Self {
        Self { pipelines }
    }

    /// Create an empty composite pipeline.
    pub fn empty() -> Self {
        Self {
            pipelines: Vec::new(),
        }
    }
}

impl Pipeline for CompositePipeline {
    fn run(&self, tree: &mut mir::NodeTree, ctx: &mut PipelineContext<'_>) -> bool {
        let mut any_changed = false;
        for pipeline in &self.pipelines {
            let changed = pipeline.run(tree, ctx);
            any_changed |= changed;
        }
        any_changed
    }

    fn name(&self) -> &'static str {
        "CompositePipeline"
    }
}

/// Builder for creating complex pipelines with an ergonomic API.
pub struct PipelineBuilder {
    pipelines: Vec<Box<dyn Pipeline>>,
}

impl std::fmt::Debug for PipelineBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PipelineBuilder")
            .field(
                "pipelines",
                &self.pipelines.iter().map(|p| p.name()).collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl PipelineBuilder {
    /// Create a new pipeline builder.
    pub fn new() -> Self {
        Self {
            pipelines: Vec::new(),
        }
    }

    /// Add a module pass.
    pub fn module_pass<P: ModulePass + 'static>(mut self, pass: P) -> Self {
        let mut pipeline = ModulePipeline::empty();
        pipeline.add(pass);
        self.pipelines.push(Box::new(pipeline));
        self
    }

    /// Add function passes (auto wrapped in FunctionToModule adaptor).
    pub fn function_passes(mut self, passes: Vec<Box<dyn FunctionPass>>) -> Self {
        let inner = FunctionPipeline::new(passes);
        self.pipelines
            .push(Box::new(FunctionToModuleAdaptor::new(inner)));
        self
    }

    /// Add a function pipeline (auto wrapped in FunctionToModule adaptor).
    pub fn function_pipeline(mut self, pipeline: FunctionPipeline) -> Self {
        self.pipelines
            .push(Box::new(FunctionToModuleAdaptor::new(pipeline)));
        self
    }

    /// Repeat an inner pipeline until fixed point.
    pub fn repeat<P: Pipeline + 'static>(mut self, max: usize, inner: P) -> Self {
        self.pipelines
            .push(Box::new(RepeatedPipeline::new(inner, max)));
        self
    }

    /// Add a raw pipeline element.
    pub fn pipeline<P: Pipeline + 'static>(mut self, pipeline: P) -> Self {
        self.pipelines.push(Box::new(pipeline));
        self
    }

    /// Build the final composite pipeline.
    pub fn build(self) -> CompositePipeline {
        CompositePipeline::new(self.pipelines)
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the optimization pipeline for the given level.
pub fn default_pipeline(level: OptimizationLevel) -> CompositePipeline {
    match level {
        OptimizationLevel::O0 => o0_pipeline(),
        OptimizationLevel::O1 => o1_pipeline(),
        OptimizationLevel::O2 => o2_pipeline(),
        OptimizationLevel::O3 => o3_pipeline(),
    }
}

// pass bundles: each returns a fresh vec of boxed passes

fn verify() -> Vec<Box<dyn FunctionPass>> {
    vec![
        Box::new(MoveCheck),
        Box::new(BorrowCheck),
        Box::new(StackCheck),
    ]
}

fn canonicalize() -> Vec<Box<dyn FunctionPass>> {
    vec![Box::new(Sroa), Box::new(Mem2Reg), Box::new(DropInsert)]
}

/// Fast simplification passes that benefit from tight iteration.
fn simplify() -> Vec<Box<dyn FunctionPass>> {
    vec![
        Box::new(ConstantFold),
        Box::new(InstructionCombine),
        Box::new(SimplifyCfg),
        Box::new(DeadCodeEliminate),
    ]
}

/// Redundancy elimination requiring analysis.
fn eliminate_redundancy() -> Vec<Box<dyn FunctionPass>> {
    vec![
        Box::new(SparseConditionalConstantPropagation),
        Box::new(Reassociate),
        Box::new(CorrelatedValueProp),
        Box::new(ValueRangePropagation),
        Box::new(Narrow),
        Box::new(GuardEliminate),
        Box::new(PartialRedundancyElim),
        Box::new(GlobalValueNumbering),
        Box::new(CodeHoisting),
        Box::new(IfConvert),
        Box::new(LocalCse),
        Box::new(CopyPropagate),
    ]
}

/// Lightweight scalar fixed point island.
fn scalar_island_light() -> Vec<Box<dyn FunctionPass>> {
    let mut passes = Vec::new();
    passes.extend(simplify());
    passes.push(Box::new(LocalCse));
    passes.push(Box::new(CopyPropagate));
    passes
}

/// Full scalar fixed point island.
fn scalar_island_full(aggressive: bool) -> Vec<Box<dyn FunctionPass>> {
    let mut passes = Vec::new();
    passes.extend(eliminate_redundancy());
    passes.extend(simplify());
    if aggressive {
        passes.push(Box::new(InstructionCombine));
        passes.push(Box::new(SimplifyCfg));
        passes.push(Box::new(DeadCodeEliminate));
    }
    passes
}

fn optimize_memory() -> Vec<Box<dyn FunctionPass>> {
    vec![
        Box::new(LoadStoreForward),
        Box::new(MemCse),
        Box::new(DeadStoreEliminate),
    ]
}

fn optimize_loops(aggressive: bool) -> Vec<Box<dyn FunctionPass>> {
    let mut passes: Vec<Box<dyn FunctionPass>> = vec![
        Box::new(LoopSimplify),
        Box::new(LoopRotate),
        Box::new(LoopPeel),
        Box::new(InductionVariableSimplify),
        Box::new(LoopStrengthReduce),
        Box::new(LoopVersioning),
        Box::new(LoopIdiomRecognize),
        Box::new(Licm),
    ];
    if aggressive {
        passes.push(Box::new(LoopUnswitch));
        passes.push(Box::new(LoopUnroll));
    }
    passes.push(Box::new(LoopDelete));
    passes
}

fn optimize_types() -> Vec<Box<dyn FunctionPass>> {
    vec![
        Box::new(LoopBoundsCheckEliminate),
        Box::new(BoundsCheckEliminate),
    ]
}

fn cleanup() -> Vec<Box<dyn FunctionPass>> {
    vec![Box::new(SimplifyCfg), Box::new(DeadCodeEliminate)]
}

/// O0: Verification and correctness only.
fn o0_pipeline() -> CompositePipeline {
    PipelineBuilder::new()
        .function_passes(verify())
        .function_passes(canonicalize())
        .build()
}

/// O1: Fast compilation with essential optimizations.
fn o1_pipeline() -> CompositePipeline {
    PipelineBuilder::new()
        .function_passes(verify())
        .function_passes(canonicalize())
        .function_passes(scalar_island_light())
        .function_passes(optimize_types())
        .function_passes(cleanup())
        .build()
}

/// O2: Release builds with comprehensive optimization.
///
/// Structure: verify → canonicalize → [simplify ↔ optimize]* → cleanup
/// Each major phase is followed by simplification to expose new opportunities.
fn o2_pipeline() -> CompositePipeline {
    PipelineBuilder::new()
        .function_passes(verify())
        .function_passes(canonicalize())
        // early scalar fixed point island
        .repeat(
            2,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_island_full(false))),
        )
        // memory optimization
        .function_passes(optimize_memory())
        .function_passes(scalar_island_full(false))
        // loop optimization
        .function_passes(optimize_loops(false))
        .repeat(
            2,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_island_full(false))),
        )
        // type optimization
        .function_passes(optimize_types())
        .function_passes(scalar_island_full(false))
        // late scalar
        .module_pass(TailCallElim)
        .function_passes(vec![Box::new(Sink)])
        .function_passes(cleanup())
        .build()
}

/// O3: Aggressive optimization.
///
/// More iterations, aggressive loop transforms, extra cleanup rounds.
fn o3_pipeline() -> CompositePipeline {
    PipelineBuilder::new()
        .function_passes(verify())
        .function_passes(canonicalize())
        // early scalar fixed point island (more iterations)
        .repeat(
            3,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_island_full(true))),
        )
        // memory optimization
        .function_passes(optimize_memory())
        .function_passes(scalar_island_full(true))
        // loop optimization (aggressive)
        .function_passes(optimize_loops(true))
        .repeat(
            2,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_island_full(true))),
        )
        // type optimization
        .function_passes(optimize_types())
        .repeat(
            2,
            FunctionToModuleAdaptor::new(FunctionPipeline::new(scalar_island_full(true))),
        )
        // late scalar
        .module_pass(TailCallElim)
        .function_passes(vec![Box::new(Sink)])
        .function_passes(cleanup())
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimize::AnalysisPreservation;

    /// Test function pass that does nothing.
    struct NoOpFunctionPass;

    impl FunctionPass for NoOpFunctionPass {
        fn run(
            &self,
            _func: &mut mir::Function,
            _tree: &mut mir::NodeTree,
            _ctx: &PipelineContext<'_>,
        ) -> AnalysisPreservation {
            AnalysisPreservation::all()
        }

        fn name(&self) -> &'static str {
            "NoOp"
        }
    }

    /// Test function pass that claims to make changes.
    #[allow(dead_code)]
    struct ChangesFunctionPass;

    impl FunctionPass for ChangesFunctionPass {
        fn run(
            &self,
            _func: &mut mir::Function,
            _tree: &mut mir::NodeTree,
            _ctx: &PipelineContext<'_>,
        ) -> AnalysisPreservation {
            AnalysisPreservation::none()
        }

        fn name(&self) -> &'static str {
            "Changes"
        }
    }

    #[test]
    fn test_function_pipeline_empty() {
        let pipeline = FunctionPipeline::empty();
        assert!(pipeline.is_empty());
        assert_eq!(pipeline.len(), 0);
    }

    #[test]
    fn test_function_pipeline_with_passes() {
        let mut pipeline = FunctionPipeline::empty();
        pipeline.add(NoOpFunctionPass);
        pipeline.add(NoOpFunctionPass);

        assert!(!pipeline.is_empty());
        assert_eq!(pipeline.len(), 2);
    }

    #[test]
    fn test_pipeline_builder() {
        let pipeline = PipelineBuilder::new()
            .function_passes(vec![Box::new(NoOpFunctionPass), Box::new(NoOpFunctionPass)])
            .build();

        // should have one pipeline element (FunctionToModule adaptor)
        assert_eq!(pipeline.pipelines.len(), 1);
    }

    #[test]
    fn test_pipeline_builder_repeat() {
        let inner = FunctionPipeline::new(vec![Box::new(NoOpFunctionPass)]);
        let pipeline = PipelineBuilder::new()
            .repeat(4, FunctionToModuleAdaptor::new(inner))
            .build();

        assert_eq!(pipeline.pipelines.len(), 1);
    }
}
