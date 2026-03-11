use destack_source::{
    ModuleId, ModuleStamp, ModuleVersion, PackageId, PackageStamp, PackageVersion, ProfileStamp,
    ProfileVersion,
};
use destack_workspace::{Module, ModuleGraphStamp, ModuleGraphVersion, ProfileId};

use crate::{Compiler, InternalError, TaskSkipError, TaskSkipReason};

impl Compiler {
    /// Get the current module version.
    pub fn module_version(&self, module_id: ModuleId) -> ModuleVersion {
        let module = self.program.modules.get(module_id);
        module.read().version
    }

    /// Get the current module stamp.
    pub fn module_stamp(&self, module_id: ModuleId) -> ModuleStamp {
        ModuleStamp::new(module_id, self.module_version(module_id))
    }

    /// Get the current profile version.
    pub fn profile_version(&self, profile_id: ProfileId) -> ProfileVersion {
        let Some(profile) = self.program.profiles.get(profile_id) else {
            self.error(InternalError::MissingProfile { profile_id });
            return ProfileVersion::INITIAL;
        };
        profile.version
    }

    /// Get the current profile stamp.
    pub fn profile_stamp(&self, profile_id: ProfileId) -> ProfileStamp {
        ProfileStamp::new(profile_id, self.profile_version(profile_id))
    }

    /// Get the current module graph version.
    pub fn module_graph_version(&self, profile_id: ProfileId) -> ModuleGraphVersion {
        self.program.module_graph_version(profile_id)
    }

    /// Get the current module graph stamp.
    pub fn module_graph_stamp(&self, profile_id: ProfileId) -> ModuleGraphStamp {
        ModuleGraphStamp::new(profile_id, self.module_graph_version(profile_id))
    }

    /// Get the current package version.
    pub fn package_version(&self, package_id: PackageId) -> PackageVersion {
        self.program.packages.version(package_id)
    }

    /// Get the current package stamp.
    pub fn package_stamp(&self, package_id: PackageId) -> PackageStamp {
        PackageStamp::new(package_id, self.package_version(package_id))
    }

    /// Ensure a module version matches the current program state.
    pub(crate) fn ensure_module_version_matches<E>(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
    ) -> Result<(), E>
    where
        E: TaskSkipError,
    {
        if self.module_version_matches(module_id, module_version) {
            Ok(())
        } else {
            Err(E::skipped(TaskSkipReason::StaleModuleVersion))
        }
    }

    /// Ensure a module version matches the current program state using a module guard.
    pub(crate) fn ensure_module_version_matches_guard<E>(
        &self,
        module: &Module,
        module_version: ModuleVersion,
    ) -> Result<(), E>
    where
        E: TaskSkipError,
    {
        if module.version == module_version {
            Ok(())
        } else {
            Err(E::skipped(TaskSkipReason::StaleModuleVersion))
        }
    }

    /// Ensure a profile version matches the current program state.
    pub(crate) fn ensure_profile_version_matches<E>(
        &self,
        profile_id: ProfileId,
        profile_version: ProfileVersion,
    ) -> Result<(), E>
    where
        E: TaskSkipError,
    {
        if self.profile_version_matches(profile_id, profile_version) {
            Ok(())
        } else {
            Err(E::skipped(TaskSkipReason::StaleProfileVersion))
        }
    }

    /// Ensure a module and profile version pair matches the current program state.
    pub(crate) fn ensure_module_profile_matches_guard<E>(
        &self,
        module: &Module,
        module_version: ModuleVersion,
        profile_id: ProfileId,
        profile_version: ProfileVersion,
    ) -> Result<(), E>
    where
        E: TaskSkipError,
    {
        self.ensure_module_version_matches_guard::<E>(module, module_version)?;
        self.ensure_profile_version_matches::<E>(profile_id, profile_version)?;
        Ok(())
    }

    /// Ensure a module graph version matches the current program state.
    pub(crate) fn ensure_module_graph_version_matches<E>(
        &self,
        profile_id: ProfileId,
        graph_version: ModuleGraphVersion,
    ) -> Result<(), E>
    where
        E: TaskSkipError,
    {
        if self.module_graph_version_matches(profile_id, graph_version) {
            Ok(())
        } else {
            Err(E::skipped(TaskSkipReason::StaleModuleGraphVersion))
        }
    }

    /// Ensure a module and profile version pair matches the current program state.
    pub(crate) fn ensure_module_profile_matches<E>(
        &self,
        module_id: ModuleId,
        module_version: ModuleVersion,
        profile_id: ProfileId,
        profile_version: ProfileVersion,
    ) -> Result<(), E>
    where
        E: TaskSkipError,
    {
        self.ensure_module_version_matches::<E>(module_id, module_version)?;
        self.ensure_profile_version_matches::<E>(profile_id, profile_version)?;
        Ok(())
    }

    /// Check whether a module version is current.
    pub(crate) fn module_version_matches(
        &self,
        module_id: ModuleId,
        expected: ModuleVersion,
    ) -> bool {
        // compare current module version
        let current = self.module_version(module_id);
        current == expected
    }

    /// Check whether a profile version is current.
    pub(crate) fn profile_version_matches(
        &self,
        profile_id: ProfileId,
        expected: ProfileVersion,
    ) -> bool {
        // compare current profile version
        let Some(profile) = self.program.profiles.get(profile_id) else {
            self.error(InternalError::MissingProfile { profile_id });
            return false;
        };
        profile.version == expected
    }

    /// Check whether a package version is current.
    pub(crate) fn package_version_matches(
        &self,
        package_id: PackageId,
        expected: PackageVersion,
    ) -> bool {
        // compare current package version
        let current = self.package_version(package_id);
        current == expected
    }

    /// Check whether a module graph version is current.
    pub(crate) fn module_graph_version_matches(
        &self,
        profile_id: ProfileId,
        expected: ModuleGraphVersion,
    ) -> bool {
        let current = self.module_graph_version(profile_id);
        current == expected
    }
}
