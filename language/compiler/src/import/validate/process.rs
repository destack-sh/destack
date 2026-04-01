use crate::timing::tags;
use crate::{Compiler, ImportError, ImportResult};
use destack_dir::{Expression, NodeTree, SymbolTable};
use destack_source::{ModuleId, ModuleVersion};

impl Compiler {
    /// Validate module "syntactic" correctness.
    pub(crate) fn import_module_validate(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        tree: &NodeTree,
        symbols: &SymbolTable,
        roots: &[destack_dir::LocalNodeId<Expression>],
        global_augmentation_scope: destack_dir::LocalScopeId,
    ) -> ImportResult<()> {
        // skip stale tasks
        self.ensure_module_version_matches::<ImportError>(module_id, module_version)?;
        let _timing = self.timing_scope(tags::IMPORT_MODULE_VALIDATE);

        // syntax-only modules stop at AST
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        let module = self.program.modules.get(module_id);
        let module = module.as_ref();
        self.validate_dependency_top_level(module, tree, roots);
        self.validate_export_local_item_names(module, tree, roots);
        self.validate_binding_conflicts(module, tree, symbols, global_augmentation_scope);
        self.validate_export_conflicts(module, tree, symbols, roots);
        Ok(())
    }
}
