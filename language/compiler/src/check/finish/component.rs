use destack_artifact::{ArtifactProjectionFingerprint, DirCheckedComponentEntry, DirCheckedModule};
use destack_source::DiagnosticCollection;

use crate::check::CheckState;
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Finish solved check state into output DIR tables and diagnostics.
    pub(in crate::check) fn finish(
        mut self,
    ) -> CompilerResult<(Vec<DirCheckedComponentEntry>, DiagnosticCollection)> {
        // finish modules before collecting diagnostics they may emit
        let modules = self.modules.keys().copied().collect::<Vec<_>>();
        let mut entries = Vec::with_capacity(modules.len());
        for module in modules {
            let checked = DirCheckedModule::from(self.finish_module(module)?);
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
        let diagnostics = self.collect_diagnostics()?;

        Ok((entries, diagnostics))
    }
}
