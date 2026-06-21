use destack_artifact::{ArtifactProjectionFingerprint, DirCheckedComponentEntry, DirCheckedModule};
use destack_source::DiagnosticCollection;

use crate::check::CheckState;
use crate::{CompilerError, CompilerResult};

impl CheckState<'_> {
    /// Finish solved check state into output DIR tables and diagnostics.
    pub(in crate::check) fn finish(
        mut self,
    ) -> CompilerResult<(Vec<DirCheckedComponentEntry>, DiagnosticCollection)> {
        // read every module's final rows
        let modules = self.modules.keys().copied().collect::<Vec<_>>();
        let mut module_rows = Vec::with_capacity(modules.len());
        for module in modules {
            module_rows.push(self.module_rows(module)?);
        }
        let diagnostics = self.collect_diagnostics()?;

        // move segments and apply the resolved rows
        let mut entries = Vec::with_capacity(module_rows.len());
        for rows in module_rows {
            let module = rows.module();
            let checked = DirCheckedModule::from(self.finish_module(rows)?);
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
