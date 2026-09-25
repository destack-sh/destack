use tspp_artifact::{DiagnosticRecord, DirDeclared};
use tspp_repository::ArtifactAttemptRecorder;
use tspp_source::ModuleId;

use crate::CompilerResult;
use crate::sema::CheckState;

impl CheckState<'_> {
    /// Run the declare pass: walk the module's own declarations and resolve them.
    pub(in crate::sema) fn run_declare(&mut self) -> CompilerResult<()> {
        let recorder = self.recorder;
        self.with_scope(|state| {
            ArtifactAttemptRecorder::breakdown_maybe(recorder, "walk", || state.walk_declarations())
        })?;
        self.commit_underivable_exports()?;

        Ok(())
    }

    /// Finish the declare pass into its artifact and diagnostics.
    pub(in crate::sema) fn finish_declare(
        mut self,
        module: ModuleId,
    ) -> CompilerResult<(DirDeclared, Vec<DiagnosticRecord>)> {
        let diagnostics = self.write_declared(module)?;

        Ok((self.into_declared(module)?, diagnostics))
    }

    /// Write the declared module back and collect its diagnostics.
    fn write_declared(&mut self, module: ModuleId) -> CompilerResult<Vec<DiagnosticRecord>> {
        self.write_back()?;
        let recorder = self.recorder;
        ArtifactAttemptRecorder::breakdown_maybe(recorder, "write", || self.write_module(module))?;

        self.collect_diagnostics()
    }
}
