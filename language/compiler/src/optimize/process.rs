use crate::{Compiler, OptimizeResult, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_source::ModuleId;
use destack_workspace::TargetId;

use super::{
    OptimizationContext, OptimizationLevel, OptimizeOptions, count_mir_size, default_pipeline,
};

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
                self.require_lower_module(module, &target)?;
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

        // build optimization pipeline for this level
        let pipeline = default_pipeline(level);
        if pipeline.function_pass_count() == 0 && pipeline.module_pass_count() == 0 {
            self.stats.record_optimize();
            return Ok(());
        }

        // get the module's MIR
        let module_ref = self.program.modules.get(module);
        let module_guard = module_ref.read();
        let mir = module_guard.mir(target);
        let mut tree = mir.tree.write();
        let strings = mir.strings.clone();

        // get options from dsconfig
        let strict_borrow_mode = self
            .program
            .with_dsconfig_options(&module_guard, |opts| opts.compiler.borrow_mode.is_strict())
            .unwrap_or(false);

        let options = OptimizeOptions {
            strict_borrow_mode,
            ..Default::default()
        };

        // count MIR size before optimization
        let before = count_mir_size(&tree);

        // create optimization context
        let context = OptimizationContext::new(&strings, options);

        // run the pipeline on all functions
        pipeline.run_on_module(&mut tree, &context);

        // count MIR size after optimization
        let after = count_mir_size(&tree);

        // record metrics
        self.stats.record_optimize();
        self.stats.record_optimize_mir(
            before.functions,
            before.instructions,
            after.instructions,
            before.blocks,
            after.blocks,
        );

        Ok(())
    }

    /// Get the optimization level for a target.
    fn optimization_level_for_target(&self, _target: &TargetId) -> OptimizationLevel {
        // NOTE #Incomplete: read from target config
        // For now, default to O1 (basic optimizations)
        OptimizationLevel::O1
    }
}
