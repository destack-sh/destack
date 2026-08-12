use destack_artifact::{DiagnosticRecord, DirDeclared};
use destack_repository::ArtifactAttemptRecorder;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::check::CheckState;

impl CheckState<'_> {
    /// Run the declare pass: walk the module's own declarations and settle them.
    pub(in crate::check) fn run_declare(&mut self) -> CompilerResult<()> {
        let recorder = self.recorder;
        self.with_scope(|state| {
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "walk", || state.walk())?;
            state.apply_derive_decorators()
        })?;
        self.derive_tagged_definitions()?;
        self.bind_underivable_exports()?;

        Ok(())
    }

    /// Finish the declare pass into its artifact and diagnostics.
    pub(in crate::check) fn finish_declare(
        mut self,
        module: ModuleId,
    ) -> CompilerResult<(DirDeclared, Vec<DiagnosticRecord>)> {
        self.write_back()?;

        let recorder = self.recorder;
        ArtifactAttemptRecorder::breakdown_maybe(recorder, "write", || self.write_module(module))?;
        let diagnostics = self.collect_diagnostics()?;

        Ok((self.into_declared(module)?, diagnostics))
    }
}
