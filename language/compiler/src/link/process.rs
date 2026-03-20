use crate::timing::tags;
use crate::{ArtifactRequirementError, Compiler, LinkError, LinkResult};

use destack_source::PackageId;
use destack_workspace::{ArtifactKey, TargetId};

impl Compiler {
    /// Build one package output.
    pub fn process_package_output(&self, package: PackageId, target: TargetId) -> LinkResult<()> {
        let package_stamp = self.package_stamp(package);
        if !self.package_version_matches(package_stamp.id, package_stamp.version) {
            return Ok(());
        }
        let artifact_key = ArtifactKey::package_output(package, target.clone());

        // reuse one persisted package output image when available
        if self
            .load_published_artifact(artifact_key.clone(), |compiler| {
                compiler.load_package_output_image(package, &target)
            })
            .is_some()
        {
            return Ok(());
        }

        let _timing = self.timing_scope(tags::LINK_TARGET);
        self.link_target(package_stamp.id, &target)?;
        let output = self
            .program
            .artifacts
            .package_output(package, &target)
            .ok_or_else(|| LinkError::Internal {
                package,
                message: format!("missing package output artifact for target '{target}'"),
            })?;
        self.store_artifact(&artifact_key, output.as_ref(), |compiler, output| {
            compiler.store_package_output_image(package, &target, output)
        });

        Ok(())
    }

    /// Require one package output artifact.
    pub fn require_package_output(
        &self,
        package: PackageId,
        target: &TargetId,
    ) -> Result<(), ArtifactRequirementError> {
        self.require_artifact(ArtifactKey::package_output(package, target.clone()))
    }
}
