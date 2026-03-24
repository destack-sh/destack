use std::path::Path;

use crate::{Compiler, LinkError, LinkResult};

use destack_artifact::ModuleArtifact;
use destack_source::{ModuleId, PackageId};
use destack_workspace::{Target, TargetId};

impl Compiler {
    /// Render one binary target module set into final output files.
    pub(crate) fn render_binary_target_files(
        &self,
        module_ids: &[ModuleId],
        package_dir: &Path,
        root_dir: Option<&Path>,
        target: &Target,
        target_id: &TargetId,
        package_id: PackageId,
    ) -> LinkResult<Vec<destack_artifact::OutputFile>> {
        let mut files = Vec::new();

        // render each generated binary artifact into final target files
        for module_id in module_ids {
            let artifact = self
                .artifacts
                .module_artifact(*module_id, target_id)
                .ok_or_else(|| LinkError::Internal {
                    package: package_id,
                    message: format!(
                        "missing module artifact for module {:?} target '{}'",
                        module_id, target_id.name
                    ),
                })?;

            let ModuleArtifact::Binary(binary) = artifact.as_ref() else {
                return Err(LinkError::Internal {
                    package: package_id,
                    message: format!(
                        "expected binary artifact for module {:?} target '{}'",
                        module_id, target_id.name
                    ),
                });
            };

            let module = self.program.modules.get(*module_id);
            let binary_files = crate::emit::render_binary_artifact_entries(
                module.as_ref(),
                binary,
                target,
                package_dir,
                root_dir,
            )
            .map_err(|error| LinkError::Internal {
                package: package_id,
                message: format!(
                    "failed to render binary artifact for module {:?}: unsupported file type {:?}",
                    module_id, error.file_type
                ),
            })?;
            files.extend(binary_files);
        }

        Ok(files)
    }
}
