use std::fmt;

use super::pipeline::{PackagePipeline, Pipeline, ProgramPipeline};
use crate::optimize::{
    OptimizationLevel, PackagePass, PackagePipelineContext, PackageWorkset, ProgramPass,
    ProgramPipelineContext, ProgramWorkset,
};

/// Pipeline that runs package passes in sequence.
pub struct PackagePassPipeline {
    passes: Vec<Box<dyn PackagePass>>,
}

impl fmt::Debug for PackagePassPipeline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackagePassPipeline")
            .field("passes", &self.passes.len())
            .finish()
    }
}

impl PackagePassPipeline {
    /// Create a new package pass pipeline.
    pub fn new(passes: Vec<Box<dyn PackagePass>>) -> Self {
        Self { passes }
    }

    /// Create an empty package pass pipeline.
    pub fn empty() -> Self {
        Self { passes: Vec::new() }
    }

    /// Add a pass to the pipeline.
    pub fn add<P: PackagePass + 'static>(&mut self, pass: P) {
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

impl PackagePipeline for PackagePassPipeline {
    fn run(&self, workset: &mut PackageWorkset, ctx: &mut PackagePipelineContext) -> bool {
        // track whether any pass changed the workset
        let mut any_changed = false;

        // run each package pass in order
        for pass in &self.passes {
            let preserved = pass.run(workset, ctx);
            if !preserved.preserves_all() {
                any_changed = true;
            }
        }

        any_changed
    }

    fn name(&self) -> &'static str {
        "PackagePassPipeline"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Pipeline that runs program passes in sequence.
pub struct ProgramPassPipeline {
    passes: Vec<Box<dyn ProgramPass>>,
}

impl fmt::Debug for ProgramPassPipeline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProgramPassPipeline")
            .field("passes", &self.passes.len())
            .finish()
    }
}

impl ProgramPassPipeline {
    /// Create a new program pass pipeline.
    pub fn new(passes: Vec<Box<dyn ProgramPass>>) -> Self {
        Self { passes }
    }

    /// Create an empty program pass pipeline.
    pub fn empty() -> Self {
        Self { passes: Vec::new() }
    }

    /// Add a pass to the pipeline.
    pub fn add<P: ProgramPass + 'static>(&mut self, pass: P) {
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

impl ProgramPipeline for ProgramPassPipeline {
    fn run(&self, workset: &mut ProgramWorkset, ctx: &mut ProgramPipelineContext) -> bool {
        // track whether any pass changed the workset
        let mut any_changed = false;

        // run each program pass in order
        for pass in &self.passes {
            let preserved = pass.run(workset, ctx);
            if !preserved.preserves_all() {
                any_changed = true;
            }
        }

        any_changed
    }

    fn name(&self) -> &'static str {
        "ProgramPassPipeline"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Program pipeline that routes each package to a level specific pipeline.
pub struct PackageLevelProgramPipeline {
    o0: PackageCompositePipeline,
    o1: PackageCompositePipeline,
    o2: PackageCompositePipeline,
    o3: PackageCompositePipeline,
    o4: PackageCompositePipeline,
}

impl fmt::Debug for PackageLevelProgramPipeline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackageLevelProgramPipeline").finish()
    }
}

impl PackageLevelProgramPipeline {
    /// Create a new package level pipeline from per level pipelines.
    pub fn new(
        o0: PackageCompositePipeline,
        o1: PackageCompositePipeline,
        o2: PackageCompositePipeline,
        o3: PackageCompositePipeline,
        o4: PackageCompositePipeline,
    ) -> Self {
        Self { o0, o1, o2, o3, o4 }
    }

    /// Get the pipeline for a level.
    pub fn pipeline_for(&self, level: OptimizationLevel) -> &PackageCompositePipeline {
        match level {
            OptimizationLevel::O0 => &self.o0,
            OptimizationLevel::O1 => &self.o1,
            OptimizationLevel::O2 => &self.o2,
            OptimizationLevel::O3 => &self.o3,
            OptimizationLevel::O4 => &self.o4,
        }
    }
}

impl ProgramPipeline for PackageLevelProgramPipeline {
    fn run(&self, workset: &mut ProgramWorkset, ctx: &mut ProgramPipelineContext) -> bool {
        // track whether any package changed
        let mut any_changed = false;

        // run the level pipeline for each package
        for package in workset.packages_mut() {
            let pipeline = self.pipeline_for(package.optimization_level());
            let changed = ctx.with_package_context(package, |package, package_ctx| {
                pipeline.run(package, package_ctx)
            });

            if changed {
                any_changed = true;
            }
        }

        any_changed
    }

    fn name(&self) -> &'static str {
        "PackageLevelProgramPipeline"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Pipeline that runs a module pipeline on each module in a package.
#[derive(Debug)]
pub struct ModuleToPackageAdaptor<P: Pipeline> {
    inner: P,
}

impl<P: Pipeline> ModuleToPackageAdaptor<P> {
    /// Create a new adaptor.
    pub fn new(inner: P) -> Self {
        Self { inner }
    }
}

impl<P: Pipeline + 'static> PackagePipeline for ModuleToPackageAdaptor<P> {
    fn run(&self, workset: &mut PackageWorkset, ctx: &mut PackagePipelineContext) -> bool {
        // track whether any module changes
        let mut any_changed = false;

        // run the module pipeline for each module
        for module in workset.modules_mut() {
            let changed = ctx.with_module_context(module, |module_ctx| {
                module.with_tree_mut(|tree| self.inner.run(tree, module_ctx))
            });

            if changed {
                any_changed = true;
            }
        }

        any_changed
    }

    fn name(&self) -> &'static str {
        "ModuleToPackage"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Pipeline that runs a package pipeline on each package in a program.
#[derive(Debug)]
pub struct PackageToProgramAdaptor<P: PackagePipeline> {
    inner: P,
}

impl<P: PackagePipeline> PackageToProgramAdaptor<P> {
    /// Create a new adaptor.
    pub fn new(inner: P) -> Self {
        Self { inner }
    }
}

impl<P: PackagePipeline + 'static> ProgramPipeline for PackageToProgramAdaptor<P> {
    fn run(&self, workset: &mut ProgramWorkset, ctx: &mut ProgramPipelineContext) -> bool {
        // track whether any package changes
        let mut any_changed = false;

        // run the package pipeline for each package
        for package in workset.packages_mut() {
            let changed = ctx.with_package_context(package, |package, package_ctx| {
                self.inner.run(package, package_ctx)
            });

            if changed {
                any_changed = true;
            }
        }

        any_changed
    }

    fn name(&self) -> &'static str {
        "PackageToProgram"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Pipeline that repeats a package pipeline until no changes or max iterations.
pub struct RepeatedPackagePipeline<P: PackagePipeline> {
    inner: P,
    max_iterations: usize,
}

impl<P: PackagePipeline> fmt::Debug for RepeatedPackagePipeline<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RepeatedPackagePipeline")
            .field("max_iterations", &self.max_iterations)
            .finish()
    }
}

impl<P: PackagePipeline> RepeatedPackagePipeline<P> {
    /// Create a new repeated package pipeline.
    pub fn new(inner: P, max_iterations: usize) -> Self {
        Self {
            inner,
            max_iterations,
        }
    }
}

impl<P: PackagePipeline + 'static> PackagePipeline for RepeatedPackagePipeline<P> {
    fn run(&self, workset: &mut PackageWorkset, ctx: &mut PackagePipelineContext) -> bool {
        // track whether any iteration changed the workset
        let mut any_changed = false;

        // repeat until a fixed point or max iterations
        for _ in 0..self.max_iterations {
            // run the inner pipeline
            let changed = self.inner.run(workset, ctx);

            // stop when no changes were made
            if !changed {
                break;
            }

            // record that something changed
            any_changed = true;
        }

        any_changed
    }

    fn name(&self) -> &'static str {
        "RepeatedPackage"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Pipeline that repeats a program pipeline until no changes or max iterations.
pub struct RepeatedProgramPipeline<P: ProgramPipeline> {
    inner: P,
    max_iterations: usize,
}

impl<P: ProgramPipeline> fmt::Debug for RepeatedProgramPipeline<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RepeatedProgramPipeline")
            .field("max_iterations", &self.max_iterations)
            .finish()
    }
}

impl<P: ProgramPipeline> RepeatedProgramPipeline<P> {
    /// Create a new repeated program pipeline.
    pub fn new(inner: P, max_iterations: usize) -> Self {
        Self {
            inner,
            max_iterations,
        }
    }
}

impl<P: ProgramPipeline + 'static> ProgramPipeline for RepeatedProgramPipeline<P> {
    fn run(&self, workset: &mut ProgramWorkset, ctx: &mut ProgramPipelineContext) -> bool {
        // track whether any iteration changed the workset
        let mut any_changed = false;

        // repeat until a fixed point or max iterations
        for _ in 0..self.max_iterations {
            // run the inner pipeline
            let changed = self.inner.run(workset, ctx);

            // stop when no changes were made
            if !changed {
                break;
            }

            // record that something changed
            any_changed = true;
        }

        any_changed
    }

    fn name(&self) -> &'static str {
        "RepeatedProgram"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Pipeline that runs multiple package pipelines in sequence.
pub struct PackageCompositePipeline {
    pipelines: Vec<Box<dyn PackagePipeline>>,
}

impl fmt::Debug for PackageCompositePipeline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackageCompositePipeline")
            .field("pipelines", &self.pipelines.len())
            .finish()
    }
}

impl PackageCompositePipeline {
    /// Create a new composite pipeline.
    pub fn new(pipelines: Vec<Box<dyn PackagePipeline>>) -> Self {
        Self { pipelines }
    }

    /// Check if the pipeline is empty.
    pub fn is_empty(&self) -> bool {
        self.pipelines.is_empty()
    }
}

impl PackagePipeline for PackageCompositePipeline {
    fn run(&self, workset: &mut PackageWorkset, ctx: &mut PackagePipelineContext) -> bool {
        // track whether any pipeline changed the workset
        let mut any_changed = false;

        // run each pipeline in order
        for pipeline in &self.pipelines {
            let changed = pipeline.run(workset, ctx);
            if changed {
                any_changed = true;
            }
        }

        any_changed
    }

    fn name(&self) -> &'static str {
        "PackageCompositePipeline"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Pipeline that runs multiple program pipelines in sequence.
pub struct ProgramCompositePipeline {
    pipelines: Vec<Box<dyn ProgramPipeline>>,
}

impl fmt::Debug for ProgramCompositePipeline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProgramCompositePipeline")
            .field("pipelines", &self.pipelines.len())
            .finish()
    }
}

impl ProgramCompositePipeline {
    /// Create a new composite pipeline.
    pub fn new(pipelines: Vec<Box<dyn ProgramPipeline>>) -> Self {
        Self { pipelines }
    }

    /// Check if the pipeline is empty.
    pub fn is_empty(&self) -> bool {
        self.pipelines.is_empty()
    }
}

impl ProgramPipeline for ProgramCompositePipeline {
    fn run(&self, workset: &mut ProgramWorkset, ctx: &mut ProgramPipelineContext) -> bool {
        // track whether any pipeline changed the workset
        let mut any_changed = false;

        // run each pipeline in order
        for pipeline in &self.pipelines {
            let changed = pipeline.run(workset, ctx);
            if changed {
                any_changed = true;
            }
        }

        any_changed
    }

    fn name(&self) -> &'static str {
        "ProgramCompositePipeline"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use destack_source::{PackageId, TargetId};

    use super::*;

    /// Package pipeline that increments a counter.
    struct CountingPackagePipeline {
        /// Counter incremented on each run.
        counter: Arc<AtomicUsize>,
        /// Whether the pipeline reports changes.
        changed: bool,
    }

    impl CountingPackagePipeline {
        /// Create a new counting pipeline.
        fn new(counter: Arc<AtomicUsize>, changed: bool) -> Self {
            Self { counter, changed }
        }
    }

    impl PackagePipeline for CountingPackagePipeline {
        fn run(&self, _workset: &mut PackageWorkset, _ctx: &mut PackagePipelineContext) -> bool {
            self.counter.fetch_add(1, Ordering::SeqCst);

            self.changed
        }

        fn name(&self) -> &'static str {
            "CountingPackage"
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    /// Program pipeline that increments a counter.
    struct CountingProgramPipeline {
        /// Counter incremented on each run.
        counter: Arc<AtomicUsize>,
        /// Whether the pipeline reports changes.
        changed: bool,
    }

    impl CountingProgramPipeline {
        /// Create a new counting pipeline.
        fn new(counter: Arc<AtomicUsize>, changed: bool) -> Self {
            Self { counter, changed }
        }
    }

    impl ProgramPipeline for CountingProgramPipeline {
        fn run(&self, _workset: &mut ProgramWorkset, _ctx: &mut ProgramPipelineContext) -> bool {
            self.counter.fetch_add(1, Ordering::SeqCst);

            self.changed
        }

        fn name(&self) -> &'static str {
            "CountingProgram"
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    /// Program pipeline that flips to no change after a set count.
    struct ToggleProgramPipeline {
        /// Counter incremented on each run.
        counter: Arc<AtomicUsize>,
        /// Runs that report changes before returning false.
        stop_after: usize,
    }

    impl ToggleProgramPipeline {
        /// Create a new toggling pipeline.
        fn new(counter: Arc<AtomicUsize>, stop_after: usize) -> Self {
            Self {
                counter,
                stop_after,
            }
        }
    }

    impl ProgramPipeline for ToggleProgramPipeline {
        fn run(&self, _workset: &mut ProgramWorkset, _ctx: &mut ProgramPipelineContext) -> bool {
            let iteration = self.counter.fetch_add(1, Ordering::SeqCst);

            iteration < self.stop_after
        }

        fn name(&self) -> &'static str {
            "ToggleProgram"
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }

    /// Build one stable test target id for one package.
    fn test_target_id(package_id: PackageId, name: &str) -> TargetId {
        TargetId::new(package_id, name)
    }

    /// Create a package workset for routing tests.
    fn package_workset(id: u64, level: OptimizationLevel) -> PackageWorkset {
        let package_id = PackageId::new(id.into());
        let target_id = test_target_id(package_id, "test");

        PackageWorkset::new(package_id, target_id, level)
    }

    /// Routes packages to pipelines matching their optimization level.
    #[test]
    fn test_package_level_program_pipeline_routes_levels() {
        let o0 = Arc::new(AtomicUsize::new(0));
        let o1 = Arc::new(AtomicUsize::new(0));
        let o2 = Arc::new(AtomicUsize::new(0));
        let o3 = Arc::new(AtomicUsize::new(0));
        let o4 = Arc::new(AtomicUsize::new(0));

        let pipeline = PackageLevelProgramPipeline::new(
            PackageCompositePipeline::new(vec![Box::new(CountingPackagePipeline::new(
                Arc::clone(&o0),
                false,
            ))]),
            PackageCompositePipeline::new(vec![Box::new(CountingPackagePipeline::new(
                Arc::clone(&o1),
                false,
            ))]),
            PackageCompositePipeline::new(vec![Box::new(CountingPackagePipeline::new(
                Arc::clone(&o2),
                false,
            ))]),
            PackageCompositePipeline::new(vec![Box::new(CountingPackagePipeline::new(
                Arc::clone(&o3),
                true,
            ))]),
            PackageCompositePipeline::new(vec![Box::new(CountingPackagePipeline::new(
                Arc::clone(&o4),
                false,
            ))]),
        );

        let mut workset = ProgramWorkset::new();
        workset.add_package(package_workset(1, OptimizationLevel::O1));
        workset.add_package(package_workset(2, OptimizationLevel::O3));

        let mut ctx = ProgramPipelineContext::new("test".to_string());
        let changed = pipeline.run(&mut workset, &mut ctx);

        assert!(changed);
        assert_eq!(o0.load(Ordering::SeqCst), 0);
        assert_eq!(o1.load(Ordering::SeqCst), 1);
        assert_eq!(o2.load(Ordering::SeqCst), 0);
        assert_eq!(o3.load(Ordering::SeqCst), 1);
        assert_eq!(o4.load(Ordering::SeqCst), 0);
    }

    /// Runs all program pipelines in order and reports changes.
    #[test]
    fn test_program_composite_pipeline_runs_all() {
        let p0 = Arc::new(AtomicUsize::new(0));
        let p1 = Arc::new(AtomicUsize::new(0));

        let pipeline = ProgramCompositePipeline::new(vec![
            Box::new(CountingProgramPipeline::new(Arc::clone(&p0), false)),
            Box::new(CountingProgramPipeline::new(Arc::clone(&p1), true)),
        ]);

        let mut workset = ProgramWorkset::new();
        workset.add_package(package_workset(1, OptimizationLevel::O1));

        let mut ctx = ProgramPipelineContext::new("test".to_string());
        let changed = pipeline.run(&mut workset, &mut ctx);

        assert!(changed);
        assert_eq!(p0.load(Ordering::SeqCst), 1);
        assert_eq!(p1.load(Ordering::SeqCst), 1);
    }

    /// Stops repeating when the inner pipeline reports no changes.
    #[test]
    fn test_repeated_program_pipeline_stops_at_fixed_point() {
        let counter = Arc::new(AtomicUsize::new(0));

        let inner = ToggleProgramPipeline::new(Arc::clone(&counter), 2);
        let pipeline = RepeatedProgramPipeline::new(inner, 10);

        let mut workset = ProgramWorkset::new();
        workset.add_package(package_workset(1, OptimizationLevel::O1));

        let mut ctx = ProgramPipelineContext::new("test".to_string());
        let changed = pipeline.run(&mut workset, &mut ctx);

        assert!(changed);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
}
