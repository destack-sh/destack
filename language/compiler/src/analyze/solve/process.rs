use std::sync::Arc;

use crate::analyze::common::InferContext;
use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeResult, Compiler};
use destack_dir::InferTable;
use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{ModuleDir, ProfileId};

impl Compiler {
    /// Phase 4: Solve infer constraints.
    pub(crate) fn analyze_module_solve(
        &self,
        dir: &mut ModuleDir,
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
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_SOLVE);

        // skip non-code modules
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        // skip analysis when module language is disabled
        if !self.module_language_allowed(module_id) {
            return Ok(());
        }

        // declaration modules have no solve-time infer table
        let module = self.program.modules.get(module_id);
        let module = module.as_ref();
        if module.language_type.is_declaration() {
            return Ok(());
        }

        let options = self.analyze_context_options_for_module(module.id);
        let Some(infer) = infer else {
            return Ok(());
        };
        // solve transient infer constraints
        let ModuleDir {
            tree,
            symbols,
            types,
            ..
        } = dir;
        let tree = tree.as_ref();
        let symbols = symbols.as_ref();
        let types = Arc::make_mut(types);
        let mut ctx = InferContext::new(&module, profile, &options, tree, symbols, types, infer);
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
