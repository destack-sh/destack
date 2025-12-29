use crate::{Compiler, ExecuteError, ExecuteResult, ModuleLowerer};

use destack_mir as mir;
use destack_workspace::TargetId;

#[allow(dead_code)]
impl Compiler {
    /// Lower a module to MIR for comptime execution.
    pub(crate) fn lower_comptime_module(
        &self,
        module: &std::sync::Arc<parking_lot::RwLock<destack_workspace::Module>>,
        profile: destack_workspace::ProfileId,
        target_id: &TargetId,
    ) -> ExecuteResult<(mir::NodeTree, destack_base::StringPool)> {
        let module_guard = module.read();
        let dir = module_guard.dir(profile);
        let module_id = module_guard.id;
        let dir_tree = dir.tree.read().clone();
        let dir_roots = dir.roots.clone();
        let symbols = dir.symbols.read().clone();
        let types = dir.types.read().clone();

        let mut lowerer = ModuleLowerer::new(
            self,
            &module_guard,
            &dir_tree,
            &dir_roots,
            &symbols,
            &types,
            target_id,
        );

        lowerer
            .lower_module()
            .map_err(|error| ExecuteError::FailedLower {
                module: module_id,
                error: Box::new(error.clone()),
                message: format!("{error}"),
            })?;

        Ok(lowerer.finish())
    }
}
