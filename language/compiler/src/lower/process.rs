use crate::{Compiler, LowerResult, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_source::ModuleId;
use destack_workspace::ModuleMir;

/// Task to lower a DIR into MIR.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Lower)]
pub enum LowerTask {
    /// Lower a module into MIR.
    #[task(code = 1, trace = "module={module} target={target}")]
    LowerModule { module: ModuleId, target: String },
}

impl Compiler {
    /// Process a lower task.
    pub fn process_lower(&self, task: LowerTask) -> LowerResult<()> {
        match task {
            LowerTask::LowerModule { module, target } => {
                self.require_elaborate_module(module)?;
                self.lower_module(module, target)?;
            }
        }
        Ok(())
    }

    /// Lower a module.
    fn lower_module(&self, module_id: ModuleId, target: String) -> LowerResult<()> {
        let module = self.program.modules.get(module_id);

        // initialize MIR for this target
        {
            let mut module = module.write();
            let version = module.version;
            // replace existing MIR for this target, if any
            module.mirs.retain(|mir| mir.target != target);
            module.mirs.push(ModuleMir::new(module_id, version, target));
        }

        Ok(())
    }

    /// Ensure a module has been lowered.
    pub fn require_lower_module(
        &self,
        module: ModuleId,
        target: impl Into<String>,
    ) -> Result<(), TaskDependencyError> {
        self.do_require_task_internal_only(LowerTask::LowerModule {
            module,
            target: target.into(),
        })
    }
}
