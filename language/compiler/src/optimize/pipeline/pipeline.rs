use std::any::Any;

use destack_mir as mir;

use crate::optimize::{
    PackagePipelineContext, PackageWorkset, PipelineContext, ProgramPipelineContext, ProgramWorkset,
};

/// A composable pipeline element.
///
/// Pipelines can be nested and combined to create complex optimization strategies.
pub trait Pipeline: Send + Sync {
    /// Run the pipeline on a module.
    ///
    /// Returns true if any changes were made.
    fn run(&self, tree: &mut mir::Tree, ctx: &mut PipelineContext<'_>) -> bool;

    /// Get the name of this pipeline.
    fn name(&self) -> &'static str;

    /// Return this pipeline as a dynamic value for downcasting.
    fn as_any(&self) -> &dyn Any;
}

/// A composable package pipeline element.
pub trait PackagePipeline: Send + Sync {
    /// Run the pipeline on a package.
    ///
    /// Returns true if any changes were made.
    fn run(&self, workset: &mut PackageWorkset, ctx: &mut PackagePipelineContext) -> bool;

    /// Get the pipeline name.
    fn name(&self) -> &'static str;

    /// Return this pipeline as a dynamic value for downcasting.
    fn as_any(&self) -> &dyn Any;
}

/// A composable program pipeline element.
pub trait ProgramPipeline: Send + Sync {
    /// Run the pipeline on a program.
    ///
    /// Returns true if any changes were made.
    fn run(&self, workset: &mut ProgramWorkset, ctx: &mut ProgramPipelineContext) -> bool;

    /// Get the pipeline name.
    fn name(&self) -> &'static str;

    /// Return this pipeline as a dynamic value for downcasting.
    fn as_any(&self) -> &dyn Any;
}
