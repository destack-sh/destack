use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::{InferTable, NodeTree, SymbolTable, TypeTable};
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::ProfileId;
impl Compiler {
    /// Phase 5: Commit solved infer table outputs and discharge obligations.
    pub(crate) fn analyze_module_commit(
        &self,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: Option<&mut InferTable>,
        module_id: ModuleId,
        profile: ProfileId,
        module_version: ModuleVersion,
        profile_version: ProfileVersion,
    ) -> AnalyzeResult<()> {
        // skip stale tasks
        self.ensure_module_profile_matches::<AnalyzeError>(
            module_id,
            module_version,
            profile,
            profile_version,
        )?;
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_COMMIT);

        // skip non-code modules
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        // skip analysis when module language is disabled
        if !self.module_language_allowed(module_id) {
            return Ok(());
        }

        // declaration modules have no commit-time infer table
        let module = self.program.modules.get(module_id);
        let module = module.as_ref();
        if module.language_type.is_declaration() {
            return Ok(());
        }

        let Some(infer) = infer else {
            return Ok(());
        };

        self.commit_solved_infer_table(tree, symbols, types, infer, module, profile)
    }
}
