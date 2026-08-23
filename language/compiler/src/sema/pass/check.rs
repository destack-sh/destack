use destack_artifact::{DiagnosticRecord, DirChecked};
use destack_repository::ArtifactAttemptRecorder;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::sema::{AnnotatedSource, CheckState};

impl CheckState<'_> {
    /// Run the check pass: infer the module's bodies against the elaborated entries.
    pub(in crate::sema) fn run_check(&mut self) -> CompilerResult<()> {
        let recorder = self.recorder;
        self.with_scope(|state| {
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "walk", || state.walk_bodies())?;
            state.induce_signature_lifetimes()?;
            state.apply_capture_directives()?;
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "decorators", || {
                state.apply_decorators()
            })
        })?;
        self.report_constant_conditions()?;
        self.report_member_conflicts()?;
        self.commit_underivable_exports()?;

        Ok(())
    }

    /// Finish the check pass into its artifact and diagnostics.
    pub(in crate::sema) fn finish_check(
        mut self,
        module: ModuleId,
        format_annotations: bool,
    ) -> CompilerResult<(DirChecked, Vec<DiagnosticRecord>, Vec<AnnotatedSource>)> {
        // format annotations from the live working state
        let annotated = match format_annotations {
            true => self.format_annotated_sources()?,
            false => Vec::new(),
        };
        self.write_back()?;

        let recorder = self.recorder;
        ArtifactAttemptRecorder::breakdown_maybe(recorder, "write", || {
            self.commit_conformances(module)?;

            self.write_module(module)
        })?;
        let diagnostics = self.collect_diagnostics()?;

        // keep only resolutions the declared stage already carries
        if let Some(stage) = self.module.declared.clone() {
            self.module.resolutions.drop_carried(&stage.resolutions);
        }

        let checked = self.into_checked(module)?;

        Ok((checked, diagnostics, annotated))
    }
}
