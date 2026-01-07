use destack_mir as mir;

use super::analysis::AnalysisPreservation;
use super::context::OptimizationContext;

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
/// Provides access to pass metadata. Specific pass types (FunctionPass, ModulePass)
/// extend this with their run methods.
pub trait Pass: Send + Sync {
    /// Get the static metadata for this pass.
    fn metadata(&self) -> &'static PassMetadata;
}

/// Trait for passes that operate on individual functions.
///
/// Function passes are run once per function and can use function-level
/// analyses from the context. They are the most common type of pass.
pub trait FunctionPass: Pass {
    /// Run this pass on a single function.
    ///
    /// Returns `AnalysisPreservation` indicating which cached analyses are still valid.
    /// Use `AnalysisPreservation::all()` if no changes were made.
    /// Use `AnalysisPreservation::none()` if the CFG or any values changed.
    fn run_on_function(
        &self,
        function: &mut mir::Function,
        tree: &mut mir::NodeTree,
        context: &OptimizationContext<'_>,
    ) -> AnalysisPreservation;
}

/// Trait for passes that operate on entire modules.
///
/// Module passes can perform cross-function analysis and transformation,
/// such as inlining, interprocedural optimization, or dead function elimination.
/// They receive the full NodeTree which contains all functions, types, and globals.
pub trait ModulePass: Pass {
    /// Run this pass on a module's MIR.
    ///
    /// The `tree` contains all MIR nodes for the module (functions, types, globals).
    /// Returns `AnalysisPreservation` indicating which cached analyses are still valid.
    fn run_on_module(
        &self,
        tree: &mut mir::NodeTree,
        context: &OptimizationContext<'_>,
    ) -> AnalysisPreservation;
}

/// A boxed function pass for dynamic dispatch.
pub type BoxedFunctionPass = Box<dyn FunctionPass>;

/// A boxed module pass for dynamic dispatch.
pub type BoxedModulePass = Box<dyn ModulePass>;

/// Create a boxed function pass from a type implementing FunctionPass.
pub fn boxed_function<P: FunctionPass + 'static>(pass: P) -> BoxedFunctionPass {
    Box::new(pass)
}

/// Create a boxed module pass from a type implementing ModulePass.
pub fn boxed_module<P: ModulePass + 'static>(pass: P) -> BoxedModulePass {
    Box::new(pass)
}
