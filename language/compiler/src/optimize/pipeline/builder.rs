use std::fmt;

use super::module::{
    CompositePipeline, FunctionPipeline, FunctionToModuleAdaptor, ModulePipeline, RepeatedPipeline,
};
use super::package::{
    ModuleToPackageAdaptor, PackageCompositePipeline, PackagePassPipeline, PackageToProgramAdaptor,
    ProgramCompositePipeline, ProgramPassPipeline, RepeatedPackagePipeline,
    RepeatedProgramPipeline,
};
use super::pipeline::{PackagePipeline, Pipeline, ProgramPipeline};
use crate::optimize::{FunctionPass, ModulePass, PackagePass, ProgramPass};

/// Builder for creating complex pipelines with an ergonomic API.
pub struct PipelineBuilder {
    pipelines: Vec<Box<dyn Pipeline>>,
}

impl fmt::Debug for PipelineBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

/// Builder for package pipelines.
pub struct PackagePipelineBuilder {
    pipelines: Vec<Box<dyn PackagePipeline>>,
}

impl fmt::Debug for PackagePipelineBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PackagePipelineBuilder")
            .field("pipelines", &self.pipelines.len())
            .finish()
    }
}

impl PackagePipelineBuilder {
    /// Create a new package pipeline builder.
    pub fn new() -> Self {
        Self {
            pipelines: Vec::new(),
        }
    }

    /// Add a package pass pipeline.
    pub fn package_passes(mut self, passes: Vec<Box<dyn PackagePass>>) -> Self {
        let pipeline = PackagePassPipeline::new(passes);
        self.pipelines.push(Box::new(pipeline));
        self
    }

    /// Add a package pass.
    pub fn package_pass<P: PackagePass + 'static>(mut self, pass: P) -> Self {
        let mut pipeline = PackagePassPipeline::empty();
        pipeline.add(pass);
        self.pipelines.push(Box::new(pipeline));
        self
    }

    /// Add a module pipeline to run across all modules in the package.
    pub fn module_pipeline<P: Pipeline + 'static>(mut self, pipeline: P) -> Self {
        self.pipelines
            .push(Box::new(ModuleToPackageAdaptor::new(pipeline)));
        self
    }

    /// Add a package pipeline element.
    pub fn pipeline<P: PackagePipeline + 'static>(mut self, pipeline: P) -> Self {
        self.pipelines.push(Box::new(pipeline));
        self
    }

    /// Add a repeated package pipeline element.
    pub fn repeat<P: PackagePipeline + 'static>(mut self, max: usize, inner: P) -> Self {
        self.pipelines
            .push(Box::new(RepeatedPackagePipeline::new(inner, max)));
        self
    }

    /// Build the composite pipeline.
    pub fn build(self) -> PackageCompositePipeline {
        PackageCompositePipeline::new(self.pipelines)
    }
}

impl Default for PackagePipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for program pipelines.
pub struct ProgramPipelineBuilder {
    pipelines: Vec<Box<dyn ProgramPipeline>>,
}

impl fmt::Debug for ProgramPipelineBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProgramPipelineBuilder")
            .field("pipelines", &self.pipelines.len())
            .finish()
    }
}

impl ProgramPipelineBuilder {
    /// Create a new program pipeline builder.
    pub fn new() -> Self {
        Self {
            pipelines: Vec::new(),
        }
    }

    /// Add a program pass pipeline.
    pub fn program_passes(mut self, passes: Vec<Box<dyn ProgramPass>>) -> Self {
        let pipeline = ProgramPassPipeline::new(passes);
        self.pipelines.push(Box::new(pipeline));
        self
    }

    /// Add a program pass.
    pub fn program_pass<P: ProgramPass + 'static>(mut self, pass: P) -> Self {
        let mut pipeline = ProgramPassPipeline::empty();
        pipeline.add(pass);
        self.pipelines.push(Box::new(pipeline));
        self
    }

    /// Add a package pipeline to run across all packages in the program.
    pub fn package_pipeline<P: PackagePipeline + 'static>(mut self, pipeline: P) -> Self {
        self.pipelines
            .push(Box::new(PackageToProgramAdaptor::new(pipeline)));
        self
    }

    /// Add a module pipeline to run across all packages and modules.
    pub fn module_pipeline<P: Pipeline + 'static>(mut self, pipeline: P) -> Self {
        let package_pipeline = ModuleToPackageAdaptor::new(pipeline);
        self.pipelines
            .push(Box::new(PackageToProgramAdaptor::new(package_pipeline)));
        self
    }

    /// Add a program pipeline element.
    pub fn pipeline<P: ProgramPipeline + 'static>(mut self, pipeline: P) -> Self {
        self.pipelines.push(Box::new(pipeline));
        self
    }

    /// Add a repeated program pipeline element.
    pub fn repeat<P: ProgramPipeline + 'static>(mut self, max: usize, inner: P) -> Self {
        self.pipelines
            .push(Box::new(RepeatedProgramPipeline::new(inner, max)));
        self
    }

    /// Build the composite pipeline.
    pub fn build(self) -> ProgramCompositePipeline {
        ProgramCompositePipeline::new(self.pipelines)
    }
}

impl Default for ProgramPipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}
