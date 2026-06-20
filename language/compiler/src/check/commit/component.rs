use destack_artifact::{ArtifactProjectionFingerprint, DirCheckedComponentEntry};
use destack_source::DiagnosticCollection;

use crate::check::CheckState;
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Commit solved check state into output DIR tables and diagnostics.
    pub(in crate::check) fn commit(
        mut self,
    ) -> CompilerResult<(Vec<DirCheckedComponentEntry>, DiagnosticCollection)> {
        // read every module's commit rows
        let modules = self.modules.keys().copied().collect::<Vec<_>>();
        let mut commits = Vec::with_capacity(modules.len());
        for module in modules {
            commits.push(self.module_commit(module)?);
        }
        let diagnostics = self.collect_diagnostics()?;

        // move segments and apply the resolved rows
        let mut entries = Vec::with_capacity(commits.len());
        for commit in commits {
            let module = commit.module();
            let checked = self.commit_module(commit)?.finish();
            let fingerprint = ArtifactProjectionFingerprint::from_serialized_payload(&checked)
                .map_err(|error| CompilerError::Internal {
                    message: format!(
                        "failed to fingerprint checked DIR payload for module {module:?}: {error}"
                    ),
                })?;

            entries.push(DirCheckedComponentEntry {
                module,
                fingerprint,
                checked,
            });
        }

        Ok((entries, diagnostics))
    }
}
