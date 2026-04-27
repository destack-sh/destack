use crate::timing::tags;
use crate::{Compiler, CompilerContext, ImportResult};
use destack_dir::{Expression, SymbolTable, Tree};
use destack_source::ModuleId;

impl Compiler {
    /// Validate module "syntactic" correctness.
    pub(crate) fn import_module_validate(
        &self,
        module_id: ModuleId,
        tree: &Tree,
        symbols: &SymbolTable,
        roots: &[destack_dir::LocalNodeId<Expression>],
        global_augmentation_scope: destack_dir::LocalScopeId,
        context: &CompilerContext<'_>,
    ) -> ImportResult<()> {
        let _timing = self.timing_scope(tags::IMPORT_MODULE_VALIDATE);

        // syntax-only modules stop at AST
        if !context.is_code_module(module_id) {
            return Ok(());
        }

        let module = context.import_module(module_id)?;
        let module = module.as_ref();

        // top-level dependency placement
        self.validate_dependency_top_level(module, tree, roots);

        // local export names
        self.validate_export_local_item_names(module, tree, roots);

        // symbol conflicts
        self.validate_binding_conflicts(context, module, tree, symbols, global_augmentation_scope);

        // export conflicts
        self.validate_export_conflicts(module, tree, symbols, roots);

        Ok(())
    }
}
