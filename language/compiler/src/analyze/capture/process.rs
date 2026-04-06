use destack_dir::{CaptureTable, NodeTree, SymbolTable};
use destack_source::ModuleId;
use destack_workspace::ProfileId;

use crate::analyze::common::TreeSymbolView;
use crate::timing::tags;
use crate::{AnalyzeResult, Compiler, CompilerContext};

impl Compiler {
    /// Post-commit pass: resolve captures for closures and nested functions.
    pub(crate) fn analyze_module_capture(
        &self,
        tree: &NodeTree,
        symbols: &SymbolTable,
        captures: &mut CaptureTable,
        module_id: ModuleId,
        profile: ProfileId,
        context: &CompilerContext<'_>,
    ) -> AnalyzeResult<()> {
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_CAPTURE);

        // skip non code modules
        if !context.is_code_module(module_id) {
            return Ok(());
        }

        // skip analysis when module language is disabled
        if !self.module_language_allowed_in_context(context, module_id) {
            return Ok(());
        }

        // load module state and phase-local tables
        let module = context.module(module_id);

        // compute capture ctx
        self.compute_module_captures(
            TreeSymbolView::new(context, module.as_ref(), profile, tree, symbols),
            captures,
        )?;

        Ok(())
    }
}
