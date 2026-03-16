use crate::timing::tags;
use crate::{Compiler, ImportError, ImportResult};
use destack_dir::{NodeTree, SymbolTable, TypeTable};
use destack_source::{ModuleId, ModuleVersion};
use destack_workspace::{ImportDir, ModuleAst};

impl Compiler {
    /// Bind a module's AST to DIR (create symbols, scopes, and base DIR).
    pub(crate) fn import_module_bind(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        ast: &ModuleAst,
        dir: &mut ImportDir,
    ) -> ImportResult<()> {
        // skip stale tasks
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let _timing = self.timing_scope(tags::IMPORT_MODULE_BIND);

        // syntax-only modules stop at AST
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        let module = self.program.modules.get(module_id);
        let mut tree = std::mem::replace(&mut dir.tree, NodeTree::new(module_id));
        let mut symbols = std::mem::replace(&mut dir.symbols, SymbolTable::new(module_id));
        let mut types = std::mem::replace(&mut dir.types, TypeTable::new(module_id));

        // bind module roots
        let roots = {
            let module = module.as_ref();
            self.bind_module_roots(&module, ast, dir, &mut tree, &mut symbols, &mut types)
        };
        dir.roots.extend(roots);

        // attach annotations
        {
            let module = module.as_ref();
            let scope = (
                dir.namespace_scope,
                symbols.get_scope_mark(dir.namespace_scope),
            );
            self.attach_annotations(
                &module,
                ast,
                dir,
                scope,
                &mut tree,
                &mut symbols,
                &mut types,
            );
        }

        // mark global augmentations (for declaration merging)
        {
            let module = module.as_ref();
            self.mark_global_augmentation_symbols(&module, &tree, &mut symbols);
        }

        dir.tree = tree;
        dir.symbols = symbols;
        dir.types = types;

        Ok(())
    }
}
