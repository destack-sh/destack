use tspp_artifact::MirOptimized;
use tspp_mir::{FunctionCache, FunctionId, ModuleCache, Mutation};

use crate::CompilerResult;

/// A transformation over one MIR module.
pub(crate) trait ModulePass: Send + Sync {
    /// Transform the module and return the changes that invalidate subsequent analyses.
    fn run(
        &self,
        module: &mut MirOptimized,
        analyses: &mut ModuleCache,
    ) -> CompilerResult<Mutation>;
}

/// A transformation over one function in a MIR module.
pub(crate) trait FunctionPass: Send + Sync {
    /// Transform the selected body and return its changed inputs.
    ///
    /// Preserve other functions, global definitions, and the selected function's signature.
    /// Update the body's entries in module tables to match the rewritten instructions.
    /// Update or invalidate analyses before querying them after a rewrite.
    fn run(
        &self,
        function: FunctionId,
        module: &mut MirOptimized,
        analyses: &mut FunctionCache,
    ) -> CompilerResult<Mutation>;
}
