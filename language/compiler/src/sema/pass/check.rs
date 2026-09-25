use tspp_artifact::{DiagnosticRecord, DirChecked};
use tspp_repository::ArtifactAttemptRecorder;
use tspp_source::ModuleId;

use crate::CompilerResult;
use crate::sema::CheckState;

impl CheckState<'_> {
    /// Run the check pass: infer the module's bodies against the elaborated entries.
    pub(in crate::sema) fn run_check(&mut self) -> CompilerResult<()> {
        // walk the bodies, then apply captures and the decorator calls over them in one scope
        let recorder = self.recorder;
        self.with_scope(|state| {
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "walk", || state.walk_bodies())?;
            state.apply_capture_directives()?;
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "decorators", || {
                state.apply_decorators()
            })
        })?;

        // report what the walk exposed, then commit the exports
        self.report_field_initializations()?;
        self.report_constant_conditions()?;
        self.report_member_conflicts()?;
        self.commit_underivable_exports()?;

        Ok(())
    }

    /// Finish the check pass into its artifact and diagnostics.
    pub(in crate::sema) fn finish_check(
        mut self,
        module: ModuleId,
    ) -> CompilerResult<(DirChecked, Vec<DiagnosticRecord>)> {
        let diagnostics = self.write_checked(module)?;

        // keep only resolutions the declared stage already carries
        if let Some(stage) = self.module.declared.clone() {
            self.module.resolutions.drop_carried(&stage.resolutions);
        }

        let checked = self.into_checked(module)?;

        Ok((checked, diagnostics))
    }

    /// Write the checked module back and collect its diagnostics.
    fn write_checked(&mut self, module: ModuleId) -> CompilerResult<Vec<DiagnosticRecord>> {
        self.write_back()?;
        let recorder = self.recorder;
        ArtifactAttemptRecorder::breakdown_maybe(recorder, "write", || self.write_module(module))?;

        self.collect_diagnostics()
    }
}
