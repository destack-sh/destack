use crate::{Compiler, ImportResult};
use destack_source::ModuleId;
use destack_workspace::ModuleDir;

impl Compiler {
    /// Bind a module's AST to DIR (create symbols, scopes, and base DIR).
    pub(crate) fn import_module_bind(&self, module: ModuleId) -> ImportResult<()> {
        self.require_import_module_parse(module)?;
        let module = self.program.modules.get(module);

        // initialize DIR
        {
            let mut module = module.write();
            module.dir_base = Some(ModuleDir::new_base(module.id, module.version));
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

        // mark global augmentations (for declaration merging)
        {
            let module = module.read();
            self.mark_global_augmentation_symbols(&module);
        }

        Ok(())
    }
}
