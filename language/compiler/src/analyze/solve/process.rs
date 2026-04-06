use crate::analyze::common::{AnalyzeIndex, InferContext};
use crate::timing::tags;
use crate::{AnalyzeResult, Compiler, CompilerContext};
use destack_dir::{InferTable, NodeTree, SymbolTable, TypeTable};
use destack_source::ModuleId;
use destack_workspace::ProfileId;

impl Compiler {
    /// Phase 4: Solve infer constraints.
    pub(crate) fn analyze_module_solve(
        &self,
        tree: &NodeTree,
        symbols: &SymbolTable,
        types: &mut TypeTable,
        infer: Option<&mut InferTable>,
        module_id: ModuleId,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> AnalyzeResult<()> {
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_SOLVE);

        // skip non-code modules
        if !context.is_code_module(module_id) {
            return Ok(());
        }

        // skip analysis when module language is disabled
        if !self.module_language_allowed_in_context(context, module_id) {
            return Ok(());
        }

        // declaration modules have no solve-time infer table
        let module = context.module(module_id);
        if module.language_type.is_declaration() {
            return Ok(());
        }

        let options = context.analyze_context_options_for_module(module.id);
        let Some(infer) = infer else {
            return Ok(());
        };
        // solve transient infer constraints
        let mut ctx = InferContext::new(
            context,
            module.as_ref(),
            profile,
            &options,
            tree,
            symbols,
            types,
            infer,
            AnalyzeIndex::default(),
        );
        {
            let (mut ctx, infer) = ctx.split_type_context_and_infer();
            self.solve_infer_table(&mut ctx, infer);
        }
        {
            let (mut ctx, infer) = ctx.split_type_context_and_infer();
            self.rewrite_inferred_type_overlays_for_instance_substitutions(&mut ctx, infer)?;
        }
        {
            let (mut ctx, infer) = ctx.split_type_context_and_infer();
            self.discharge_projection_obligations_in_solve(&mut ctx, infer)?;
        }
        self.discharge_missing_member_obligations_in_solve(&mut ctx.reborrow())?;
        {
            let (mut ctx, infer) = ctx.split_type_context_and_infer();
            self.discharge_relation_obligations_in_solve(&mut ctx, infer)?;
        }

        Ok(())
    }
}
