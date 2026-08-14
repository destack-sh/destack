use destack_artifact::{DiagnosticRecord, DirDeclared};
use destack_repository::ArtifactAttemptRecorder;
use destack_source::ModuleId;

use crate::CompilerResult;
use crate::sema::CheckState;

impl CheckState<'_> {
    /// Run the declare pass: walk the module's own declarations and settle them.
    pub(in crate::sema) fn run_declare(&mut self) -> CompilerResult<()> {
        let recorder = self.recorder;
        self.with_scope(|state| {
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "walk", || state.walk())
        })?;
        self.bind_underivable_exports()?;

        Ok(())
    }

    /// Finish the declare pass into its artifact and diagnostics.
    pub(in crate::sema) fn finish_declare(
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
