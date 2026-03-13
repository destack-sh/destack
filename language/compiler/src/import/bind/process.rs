use crate::timing::tags;
use crate::{Compiler, ImportError, ImportResult};
use destack_source::{ModuleId, ModuleVersion};

impl Compiler {
    /// Bind a module's AST to DIR (create symbols, scopes, and base DIR).
    pub(crate) fn import_module_bind(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
    ) -> ImportResult<()> {
        // skip stale tasks
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let _timing = self.timing_scope(tags::IMPORT_MODULE_BIND);

        // syntax-only modules stop at AST
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        let module = self.program.modules.get(module_id);

        // bind module roots
        let roots = {
            let module = module.read();
            self.bind_module_roots(&module, module.ast())
        };
        self.with_active_base_dir_mut(module_id, |dir| {
            dir.roots.extend(roots);
        });

        // attach annotations
        {
            let module = module.read();
            let ast = module.ast();
            self.with_active_base_dir(module_id, |dir| {
                let mut tree = dir.tree.write();
                let mut symbols = dir.symbols.write();
                let mut types = dir.types.write();
                let scope = (
                    dir.namespace_scope,
                    symbols.get_scope_mark(dir.namespace_scope),
                );
                self.attach_annotations(&module, ast, scope, &mut tree, &mut symbols, &mut types);
            });
        }

        // mark global augmentations (for declaration merging)
        {
            let module = module.read();
            self.mark_global_augmentation_symbols(&module);
        }

        Ok(())
    }
}
