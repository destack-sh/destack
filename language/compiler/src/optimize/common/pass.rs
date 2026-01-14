use destack_mir as mir;

use crate::optimize::{
    AnalysisPreservation, PackagePipelineContext, PackageWorkset, PipelineContext,
    ProgramPipelineContext, ProgramWorkset,
};

/// Optimization level.
///
/// Controls which passes run and how aggressive they are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum OptimizationLevel {
    /// Debug: no optimizations.
    #[default]
    O0,
    /// Comptime/dev: fast local passes (constant fold, DCE, simplify CFG).
    O1,
    /// Release: full suite (inlining, escape analysis, devirtualization).
    O2,
    /// Hot paths: aggressive thresholds, loop unrolling.
    O3,
    /// Maximal optimization with extra fixed point rounds and optional LTO.
    O4,
}

impl std::str::FromStr for OptimizationLevel {
    type Err = ();

    /// Parse from string (e.g., "O2" or "2").
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "O0" | "0" => Ok(Self::O0),
            "O1" | "1" => Ok(Self::O1),
            "O2" | "2" => Ok(Self::O2),
            "O3" | "3" => Ok(Self::O3),
            "O4" | "4" => Ok(Self::O4),
            _ => Err(()),
        }
    }
}

/// Static metadata about an optimization pass.
#[derive(Debug, Clone, Copy)]
pub struct PassMetadata {
    /// Pass ID like "constant-fold".
    pub id: &'static str,
    /// Name like "ConstantFold".
    pub name: &'static str,
    /// Human-readable description.
    pub description: &'static str,
}

/// Base trait for all optimization passes.
///
/// Provides access to pass metadata.
pub trait Pass: Send + Sync {
    /// Get the static metadata for this pass.
    fn metadata(&self) -> &'static PassMetadata;
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

    /// Get the pass ID.
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

    /// Get the pass ID.
    fn id(&self) -> &'static str {
        self.name()
    }
}

/// Trait for passes that operate on packages.
///
/// Returns AnalysisPreservation to indicate what analyses are still valid.
pub trait PackagePass: Send + Sync {
    /// Run the pass on a package workset.
    fn run(
        &self,
        workset: &mut PackageWorkset,
        ctx: &PackagePipelineContext,
    ) -> AnalysisPreservation;

    /// Get the pass name.
    fn name(&self) -> &'static str;

    /// Get the pass ID.
    fn id(&self) -> &'static str {
        self.name()
    }
}

/// Trait for passes that operate on programs.
///
/// Returns AnalysisPreservation to indicate what analyses are still valid.
pub trait ProgramPass: Send + Sync {
    /// Run the pass on a program workset.
    fn run(
        &self,
        workset: &mut ProgramWorkset,
        ctx: &ProgramPipelineContext,
    ) -> AnalysisPreservation;

    /// Get the pass name.
    fn name(&self) -> &'static str;

    /// Get the pass ID.
    fn id(&self) -> &'static str {
        self.name()
    }
}
