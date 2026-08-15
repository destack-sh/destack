use std::fmt;

use crate::optimize::{
    FunctionPass, MirOptimized, ModulePass, PipelineContext, run_function_passes,
};
use destack_mir as mir;

use super::pipeline::Pipeline;

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

    /// Return the passes in this pipeline.
    pub fn passes(&self) -> &[Box<dyn FunctionPass>] {
        &self.passes
    }
}

impl Pipeline for FunctionPipeline {
    fn run(&self, optimized: &mut MirOptimized, ctx: &mut PipelineContext<'_>) -> bool {
        let mut any_changed = false;

        // collect function ids
        let function_ids: Vec<_> = optimized
            .tree
            .iter_nodes::<mir::Function>()
            .map(|(id, _)| id)
            .collect();

        for function_id in function_ids {
            let passes = self.passes.iter().map(|pass| pass.as_ref());
            any_changed |= run_function_passes(function_id, optimized, ctx, passes);
        }

        any_changed
    }

    fn name(&self) -> &'static str {
        "FunctionPipeline"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
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

    /// Return the passes in this pipeline.
    pub fn passes(&self) -> &[Box<dyn ModulePass>] {
        &self.passes
    }
}

impl Pipeline for ModulePipeline {
    fn run(&self, optimized: &mut MirOptimized, ctx: &mut PipelineContext<'_>) -> bool {
        let mut any_changed = false;

        // retain analyses across the module pass sequence
        let mut analyses = mir::AnalysisCache::with_options(ctx.options.analysis);

        for pass in &self.passes {
            let mutation = pass.run(optimized, ctx, &mut analyses);

            // drop the analyses this pass's mutation invalidates
            analyses.invalidate(mutation);
            if !mutation.is_none() {
                any_changed = true;
            }
        }

        any_changed
    }

    fn name(&self) -> &'static str {
        "ModulePipeline"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
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

    /// Return the wrapped function pipeline.
    pub fn inner(&self) -> &FunctionPipeline {
        &self.inner
    }
}

impl Pipeline for FunctionToModuleAdaptor {
    fn run(&self, optimized: &mut MirOptimized, ctx: &mut PipelineContext<'_>) -> bool {
        self.inner.run(optimized, ctx)
    }

    fn name(&self) -> &'static str {
        "FunctionToModule"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Pipeline that repeats an inner pipeline until no changes or max iterations.
pub struct RepeatedPipeline {
    inner: Box<dyn Pipeline>,
    max_iterations: usize,
}

impl fmt::Debug for RepeatedPipeline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RepeatedPipeline")
            .field("max_iterations", &self.max_iterations)
            .finish()
    }
}

impl RepeatedPipeline {
    /// Create a new repeated pipeline.
    pub fn new<P: Pipeline + 'static>(inner: P, max_iterations: usize) -> Self {
        Self {
            inner: Box::new(inner),
            max_iterations,
        }
    }

    /// Return the inner pipeline.
    pub fn inner(&self) -> &dyn Pipeline {
        self.inner.as_ref()
    }

    /// Return the maximum iterations.
    pub fn max_iterations(&self) -> usize {
        self.max_iterations
    }
}

impl Pipeline for RepeatedPipeline {
    fn run(&self, optimized: &mut MirOptimized, ctx: &mut PipelineContext<'_>) -> bool {
        let mut any_changed = false;

        for _ in 0..self.max_iterations {
            let changed = self.inner.run(optimized, ctx);
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
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

    /// Return the child pipelines.
    pub fn pipelines(&self) -> &[Box<dyn Pipeline>] {
        &self.pipelines
    }
}

impl Pipeline for CompositePipeline {
    fn run(&self, optimized: &mut MirOptimized, ctx: &mut PipelineContext<'_>) -> bool {
        let mut any_changed = false;
        for pipeline in &self.pipelines {
            let changed = pipeline.run(optimized, ctx);
            any_changed |= changed;
        }
        any_changed
    }

    fn name(&self) -> &'static str {
        "CompositePipeline"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use destack_mir::Mutation;

    use super::*;
    use crate::optimize::{Pass, PassMetadata, PipelineBuilder};

    /// Return a no op function pass.
    struct NoOpFunctionPass;

    /// No op pass metadata.
    static NO_OP_METADATA: PassMetadata = PassMetadata {
        id: "no-op",
        name: "NoOpFunctionPass",
        description: "No op pass.",
    };

    impl Pass for NoOpFunctionPass {
        fn metadata(&self) -> &'static PassMetadata {
            &NO_OP_METADATA
        }
    }

    impl FunctionPass for NoOpFunctionPass {
        fn run(
            &self,
            _func: &mut mir::Function,
            _optimized: &mut MirOptimized,
            _ctx: &PipelineContext<'_>,
            _analyses: &mut mir::FunctionCache,
        ) -> Mutation {
            Mutation::NONE
        }
    }

    /// Validate empty function pipeline tables.
    #[test]
    fn test_function_pipeline_empty() {
        let pipeline = FunctionPipeline::empty();
        assert!(pipeline.is_empty());
        assert_eq!(pipeline.len(), 0);
    }

    /// Validate function pipeline length tracking.
    #[test]
    fn test_function_pipeline_with_passes() {
        let mut pipeline = FunctionPipeline::empty();
        pipeline.add(NoOpFunctionPass);
        pipeline.add(NoOpFunctionPass);

        assert!(!pipeline.is_empty());
        assert_eq!(pipeline.len(), 2);
    }

    /// Validate builder inserts a function adaptor pipeline.
    #[test]
    fn test_pipeline_builder() {
        let pipeline = PipelineBuilder::new()
            .function_passes(vec![Box::new(NoOpFunctionPass), Box::new(NoOpFunctionPass)])
            .build();

        assert_eq!(pipeline.pipelines.len(), 1);
    }

    /// Validate builder repeat adds a single pipeline wrapper.
    #[test]
    fn test_pipeline_builder_repeat() {
        let inner = FunctionPipeline::new(vec![Box::new(NoOpFunctionPass)]);
        let pipeline = PipelineBuilder::new()
            .repeat(4, FunctionToModuleAdaptor::new(inner))
            .build();

        assert_eq!(pipeline.pipelines.len(), 1);
    }
}
