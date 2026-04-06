use crate::timing::tags;
use crate::{AnalyzeResult, Compiler, CompilerContext};
use destack_dir::{InferTable, NodeTree, SymbolTable, TypeTable};
use destack_source::ModuleId;
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
        context: &CompilerContext<'_>,
    ) -> AnalyzeResult<()> {
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_COMMIT);

        // skip non-code modules
        if !context.is_code_module(module_id) {
            return Ok(());
        }

        // skip analysis when module language is disabled
        if !self.module_language_allowed_in_context(context, module_id) {
            return Ok(());
        }

        // declaration modules have no commit-time infer table
        let module = context.module(module_id);
        if module.language_type.is_declaration() {
            return Ok(());
        }

        let Some(infer) = infer else {
            return Ok(());
        };

        self.commit_solved_infer_table(
            tree,
            symbols,
            types,
            infer,
            module.as_ref(),
            profile,
            context,
        )
    }
}
