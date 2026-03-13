use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::{ModuleDir, ProfileId};

use crate::analyze::common::TreeSymbolView;
use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeResult, Compiler};

impl Compiler {
    /// Post-commit pass: resolve captures for closures and nested functions.
    pub(crate) fn analyze_module_capture(
        &self,
        dir: &ModuleDir,
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
        let _timing = self.timing_scope(tags::ANALYZE_MODULE_CAPTURE);

        // skip non code modules
        if !self.is_code_module(module_id) {
            return Ok(());
        }

        // skip analysis when module language is disabled
        if !self.module_language_allowed(module_id) {
            return Ok(());
        }

        // load module state and dir ctx
        let module = self.program.modules.get(module_id);
        let module = module.read();
        let tree = dir.tree.read();
        let symbols = dir.symbols.read();
        let mut captures = dir.captures.write();

        // compute capture ctx
        self.compute_module_captures(
            TreeSymbolView::new(&module, profile, &tree, &symbols),
            &mut captures,
        )?;

        Ok(())
    }
}
