use destack_source::{ModuleId, ModuleVersion, ProfileVersion};
use destack_workspace::ProfileId;

use crate::analyze::common::TreeSymbolView;
use crate::timing::tags;
use crate::{AnalyzeError, AnalyzeResult, Compiler, TaskDependencyError};

impl Compiler {
    /// Ensure a module's captures have been analyzed.
    pub fn require_analyze_module_capture(
        &self,
        module: ModuleId,
        profile: ProfileId,
    ) -> Result<(), TaskDependencyError> {
        use crate::AnalyzeTask;
        let module = self.module_stamp(module);
        let profile = self.profile_stamp(profile);
        self.do_require_task_internal_only(AnalyzeTask::AnalyzeModuleCapture { module, profile })
    }

    /// Post-commit pass: resolve captures for closures and nested functions.
    pub(crate) fn analyze_module_capture(
        &self,
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

        // ensure dependencies are ready
        self.require_analyze_module_commit(module_id, profile)?;

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
        let dir = module.dir(profile);
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
