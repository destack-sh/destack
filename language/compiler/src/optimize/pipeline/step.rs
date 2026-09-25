use tspp_artifact::MirOptimized;
use tspp_mir::{Function, ModuleCache, Mutation};

use crate::CompilerResult;

use super::{FunctionPass, ModulePass};

/// A module transformation or a group of function transformations in execution order.
pub(in crate::optimize) enum Step<'a> {
    /// Transform the module once.
    #[expect(dead_code, reason = "No module transformations are scheduled.")]
    Module(&'a dyn ModulePass),
    /// Run the entire group on each defined function.
    Functions(&'a [&'a dyn FunctionPass]),
}

impl Step<'_> {
    /// Execute this step and invalidate its changed analyses.
    pub(super) fn run(
        &self,
        module: &mut MirOptimized,
        analyses: &mut ModuleCache,
    ) -> CompilerResult<()> {
        // select the transformation scope
        match self {
            Self::Module(pass) => {
                // invalidate analyses across the module after the transformation
                let mutation = pass.run(module, analyses)?;
                analyses.invalidate(mutation);
            }
            Self::Functions(passes) => {
                // skip function enumeration when the group is empty
                if passes.is_empty() {
                    return Ok(());
                }

                // select the bodies present after the preceding module transformations
                let functions: Vec<_> = module
                    .tree()
                    .iter_nodes::<Function>()
                    .filter(|(_, function)| function.is_defined())
                    .map(|(function, _)| function)
                    .collect();
                let mut mutation = Mutation::NONE;

                // finish each function's transformations while its analyses remain cached
                for function in functions {
                    // discard results computed from mutable module analyses
                    let cache = analyses.function(function);
                    cache.invalidate_module_dependencies();

                    // invalidate body results between successive transformations
                    for pass in *passes {
                        let changed = pass.run(function, module, cache)?;
                        cache.invalidate(changed);
                        mutation = mutation.union(changed);
                    }
                }

                // invalidate module results and dependent function results once for the group
                analyses.invalidate_module(mutation);
            }
        }

        Ok(())
    }
}
