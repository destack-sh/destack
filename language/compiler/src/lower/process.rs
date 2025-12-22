use crate::{Compiler, LowerResult, ModuleLowerer, TaskDependencyError};

use destack_compiler_macros::DefineTask;
use destack_source::ModuleId;
use destack_workspace::ModuleMir;

/// Task to lower a DIR into MIR.
#[derive(Debug, Clone, Hash, PartialEq, Eq, DefineTask)]
#[phase(Lower)]
pub enum LowerTask {
    /// Lower a module into MIR.
    #[task(code = 1, trace = "module={module} target={target}")]
    LowerModule {
        /// Identify the module to lower.
        module: ModuleId,
        /// Identify the target backend for lowering.
        target: String,
    },
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
            module
                .mirs
                .push(ModuleMir::new(module_id, version, target.clone()));
        }

        // lower the module
        let (mir_tree, mir_strings) = {
            let module = self.program.modules.get(module_id);
            let module = module.read();
            let dir = module.dir();
            let dir_tree = dir.tree.read();
            let symbols = dir.symbols.read();
            let types = dir.types.read();

            let mut lowerer = ModuleLowerer::new(
                self, &module, &dir_tree, &dir.roots, &symbols, &types, &target,
            );
            lowerer.lower_module()?;
            lowerer.finish()
        };

        // update the module with the new lowered MIR
        // (#Cleanup: should we mutate the ModuleMir in place..?)
        let module = self.program.modules.get(module_id);
        let mut module = module.write();
        let mir = module
            .mirs
            .iter_mut()
            .find(|mir| mir.target == target)
            .expect("missing ModuleMir for target");
        *mir.tree.write() = mir_tree;
        mir.strings = mir_strings;

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
