use crate::{Compiler, OptimizeResult, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_mir::Function;
use destack_source::ModuleId;
use destack_workspace::TargetId;

use super::{BoxedFunctionPass, OptimizationContext, OptimizationLevel, passes};

/// Task to optimize something.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Optimize)]
pub enum OptimizeTask {
    /// Optimize a module's MIR.
    #[task(code = 1, trace = "module={module} target={target}")]
    OptimizeModule { module: ModuleId, target: TargetId },
}

impl Compiler {
    /// Process an optimize task.
    pub fn process_optimize(&self, task: OptimizeTask) -> OptimizeResult<()> {
        match task {
            OptimizeTask::OptimizeModule { module, target } => {
                self.require_verify_module(module, &target)?;
                self.optimize_module(module, &target)?;
            }
        }
        Ok(())
    }

    /// Ensure a module has been optimized.
    pub fn require_optimize(
        &self,
        module: ModuleId,
        target: &TargetId,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(OptimizeTask::OptimizeModule {
            module,
            target: target.clone(),
        })
    }

    /// Optimize a module's MIR.
    fn optimize_module(&self, module: ModuleId, target: &TargetId) -> OptimizeResult<()> {
        // get optimization level from target config
        let level = self.optimization_level_for_target(target);

        // get the passes for this level
        let passes = passes::passes_for_level(level);
        if passes.is_empty() {
            return Ok(());
        }

        // get the module's MIR
        let module_ref = self.program.modules.get(module);
        let module_guard = module_ref.read();
        let mir = module_guard.mir(target);
        let mut tree = mir.tree.write();
        let strings = mir.strings.clone();

        // collect function IDs (to avoid borrow issues during iteration)
        let function_ids: Vec<_> = tree.iter_nodes::<Function>().map(|(id, _)| id).collect();

        // run passes on each function
        for function_id in function_ids {
            self.optimize_function(function_id, &mut tree, &strings, &passes);
        }

        Ok(())
    }

    /// Run optimization passes on a single function.
    fn optimize_function(
        &self,
        function_id: destack_mir::LocalNodeId<Function>,
        tree: &mut destack_mir::NodeTree,
        strings: &destack_base::StringPool,
        passes: &[BoxedFunctionPass],
    ) {
        // clone function to work around borrow checker
        // (Function is small - just metadata + block/local IDs)
        let mut function = tree.get(function_id).clone();

        // skip imported functions (no body to optimize)
        if function.entry.is_none() {
            return;
        }

        // create context for this function
        let context = OptimizationContext::new(strings);

        // run each pass
        for pass in passes {
            let preserved = pass.run_on_function(&mut function, tree, &context);
            context.invalidate(&preserved);
        }

        // write function back (in case passes modified it)
        *tree.get_mut(function_id) = function;
    }

    /// Get the optimization level for a target.
    fn optimization_level_for_target(&self, _target: &TargetId) -> OptimizationLevel {
        // NOTE #Incomplete: read from target config
        // For now, default to O1 (basic optimizations)
        OptimizationLevel::O1
    }
}
