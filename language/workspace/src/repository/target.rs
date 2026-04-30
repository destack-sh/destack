use destack_source::{FileId, PackageId, TargetId};

use crate::repository::{Repository, RepositoryError, Revision};
use crate::{ModuleDetection, ModuleFormat, Target};

impl Repository {
    /// Return one exact revision-scoped target by id when present.
    pub fn target(
        &self,
        revision: Revision,
        target_id: TargetId,
    ) -> Result<Option<Target>, RepositoryError> {
        let Some(package) = self.package(revision, target_id.package_id())? else {
            return Ok(None);
        };

        Ok(package.targets.get(&target_id).cloned())
    }

    /// Return one effective revision-scoped target by id when present.
    pub fn effective_target(
        &self,
        revision: Revision,
        target_id: TargetId,
    ) -> Result<Option<Target>, RepositoryError> {
        // explicit targets
        if let Some(target) = self.target(revision, target_id)? {
            return Ok(Some(target));
        }

        // implicit targets
        Ok(Target::implicit_for_id(target_id))
    }

    /// Return the package default target when one is selected by configuration.
    pub fn package_default_target(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Option<(TargetId, Target)>, RepositoryError> {
        let Some(package) = self.package(revision, package_id)? else {
            return Err(RepositoryError::MissingPackage {
                package: package_id,
            });
        };
        let package_options = self.package_options(revision, package_id)?;

        // honor one explicit default target from config
        if let Some(package_options) = package_options.as_ref()
            && let Some(default_target) = package_options.default_target.as_ref()
        {
            let target_id = TargetId::new(package_id, default_target);
            if let Some(target) = package.targets.get(&target_id) {
                return Ok(Some((target_id, target.clone())));
            }

            return Err(RepositoryError::MissingTarget { target: target_id });
        }

        // no configured targets
        if package.targets.is_empty() {
            return Ok(None);
        }

        // one configured target is the implicit default
        if let Some((target_id, target)) = package.targets.iter().next()
            && package.targets.len() == 1
        {
            return Ok(Some((*target_id, target.clone())));
        }

        // otherwise the package is ambiguous
        Ok(None)
    }

    /// Return the module detection mode from one tsconfig file.
    pub(crate) fn tsconfig_module_detection(
        &self,
        revision: Revision,
        tsconfig_file_id: Option<FileId>,
    ) -> Result<ModuleDetection, RepositoryError> {
        let Some(tsconfig_file_id) = tsconfig_file_id else {
            return Ok(ModuleDetection::default());
        };

        let Some(tsconfig) = self.tsconfig_declaration_for_file(revision, tsconfig_file_id)? else {
            return Ok(ModuleDetection::default());
        };
        let options = tsconfig.options();

        Ok(options.compiler.module_detection)
    }

    /// Return the module format override from one tsconfig file.
    pub(crate) fn tsconfig_module_format(
        &self,
        revision: Revision,
        tsconfig_file_id: Option<FileId>,
    ) -> Result<Option<ModuleFormat>, RepositoryError> {
        let Some(tsconfig_file_id) = tsconfig_file_id else {
            return Ok(None);
        };

        let Some(tsconfig) = self.tsconfig_declaration_for_file(revision, tsconfig_file_id)? else {
            return Ok(None);
        };
        let module_target = tsconfig.options().compiler.module;

        Ok(ModuleFormat::from_tsconfig_target(module_target))
    }
}
