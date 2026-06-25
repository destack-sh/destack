use destack_artifact::{ArtifactProjectionFingerprint, DirCheckedComponentEntry, DirCheckedModule};
use destack_source::DiagnosticCollection;

use crate::check::CheckState;
use crate::{CompilerError, CompilerResult};

use super::CheckedModuleSegments;

impl CheckState<'_> {
    /// Write solved check state into checked DIR artifacts and diagnostics.
    pub(in crate::check) fn write(
        mut self,
    ) -> CompilerResult<(Vec<DirCheckedComponentEntry>, DiagnosticCollection)> {
        let modules = self.modules.keys().copied().collect::<Vec<_>>();
        for module in modules.iter().copied() {
            self.write_module(module)?;
        }

        let diagnostics = self.collect_diagnostics()?;

        let mut entries = Vec::with_capacity(modules.len());
        for module in modules {
            let state = self.take_module(module);
            let checked = DirCheckedModule::from(CheckedModuleSegments::from_state(state));
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
