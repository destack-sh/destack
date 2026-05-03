use crate::{Compiler, CompilerResult};
use destack_core::StringPool;
use destack_dir::{Expression, LocalNodeId, LocalScopeId, SymbolTable, Tree};
use destack_source::ModuleId;
use destack_workspace::{ProfileId, ProviderContext};

use super::super::ImportState;

impl Compiler {
    /// Validate module "syntactic" correctness.
    pub(crate) fn import_module_validate(
        &self,
        module_id: ModuleId,
        _profile_id: ProfileId,
        tree: &Tree,
        strings: &StringPool,
        symbols: &SymbolTable,
        roots: &[LocalNodeId<Expression>],
        global_augmentation_scope: LocalScopeId,
        context: &dyn ProviderContext,
    ) -> CompilerResult<()> {
        // syntax-only modules stop at AST
        if !self.is_code_module(context.revision(), module_id) {
            return Ok(());
        }

        let module = self.module(context.revision(), module_id);
        let module = module.as_ref();

        let state = ImportState::new(
            self,
            context,
            module,
            tree,
            strings,
            symbols,
            roots,
            global_augmentation_scope,
        );

        // top-level dependency placement
        self.validate_dependency_top_level(&state)?;

        // local export names
        self.validate_export_local_item_names(&state)?;

        // symbol conflicts
        self.validate_binding_conflicts(&state)?;

        // export conflicts
        self.validate_export_conflicts(&state)?;

        Ok(())
    }
}
