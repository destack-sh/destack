use crate::timing::tags;
use crate::{BuildKey, BuildRequirementError, Compiler, LinkResult};

use destack_source::PackageId;
use destack_workspace::{OutputKey, TargetId};

impl Compiler {
    /// Build one package output.
    pub fn process_package_output(&self, package: PackageId, target: TargetId) -> LinkResult<()> {
        let package_stamp = self.package_stamp(package);
        if !self.package_version_matches(package_stamp.id, package_stamp.version) {
            return Ok(());
        }
        let _timing = self.timing_scope(tags::LINK_TARGET);
        self.link_target(package_stamp.id, &target)
    }

    /// Require one package output build product.
    pub fn require_package_output(
        &self,
        package: PackageId,
        target: &TargetId,
    ) -> Result<(), BuildRequirementError> {
        self.require_build_key(BuildKey::Output(OutputKey::package(
            package,
            target.clone(),
        )))
    }
}
