use crate::timing::tags;
use crate::{Compiler, ImportError, ImportResult};
use destack_artifact::Ast;
use destack_dir::{LocalScopeId, ModuleBinding, NodeTree, SymbolTable, TypeTable};
use destack_source::{ModuleId, ModuleVersion};

impl Compiler {
    /// Bind a module's AST to DIR (create symbols, scopes, and base DIR).
    pub(crate) fn import_module_bind(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        ast: &Ast,
        namespace_scope: LocalScopeId,
        global_augmentation_scope: LocalScopeId,
        module_bindings: &mut Vec<ModuleBinding>,
        tree: &mut NodeTree,
        symbols: &mut SymbolTable,
        types: &mut TypeTable,
        roots: &mut Vec<destack_dir::LocalNodeId<destack_dir::Expression>>,
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
        let bound_roots = {
            let module = module.as_ref();
            self.bind_module_roots(
                module,
                ast,
                namespace_scope,
                global_augmentation_scope,
                module_bindings,
                tree,
                symbols,
                types,
            )
        };
        roots.extend(bound_roots);

        // attach annotations
        {
            let module = module.as_ref();
            let scope = (namespace_scope, symbols.get_scope_mark(namespace_scope));
            self.attach_annotations(
                module,
                ast,
                scope,
                namespace_scope,
                global_augmentation_scope,
                module_bindings,
                tree,
                symbols,
                types,
            );
        }

        // mark global augmentations (for declaration merging)
        {
            let module = module.as_ref();
            self.mark_global_augmentation_symbols(module, tree, symbols);
        }

        Ok(())
    }
}
