use crate::{BindResult, Compiler, TaskDependencyError};
use destack_source::ModuleId;
use destack_workspace::ModuleDirBase;

impl Compiler {
    /// Ensure a module has been bound (DIR built).
    pub fn require_bind_module_build(&self, module: ModuleId) -> Result<(), TaskDependencyError> {
        use crate::BindTask;
        self.do_require_task_internal_only(BindTask::BindModuleBuild { module })
    }

    /// Build DIR for a module by binding its AST.
    pub(crate) fn bind_module_build(&self, module: ModuleId) -> BindResult<()> {
        self.require_import_module(module)?;
        let module = self.program.modules.get(module);

        // initialize DIR
        {
            let mut module = module.write();
            module.dir_base = Some(ModuleDirBase::new(module.id, module.version));
            module.dirs.clear();
        }

        // bind module roots
        let roots = {
            let module = module.read();
            self.bind_module_roots(&module, module.ast())
        };
        {
            let mut module = module.write();
            module.dir_base_mut().roots.extend(roots);
        };

        // attach annotations
        {
            let module = module.read();
            let dir = module.dir_base();
            let ast = module.ast();
            let mut tree = dir.tree.write();
            let mut symbols = dir.symbols.write();
            let mut types = dir.types.write();
            let scope = (
                dir.namespace_scope,
                symbols.get_scope_mark(dir.namespace_scope),
            );
            self.attach_annotations(&module, ast, scope, &mut tree, &mut symbols, &mut types);
        }

        // bind module exports
        {
            let mut module = module.write();
            self.bind_module_exports(&mut module);
        }

        Ok(())
    }
}
