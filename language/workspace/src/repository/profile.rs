use std::sync::Arc;

use destack_artifact::ProfileKey;
use destack_source::{ModuleId, PackageId, ProfileId, TargetId};
use im::OrdMap;

use crate::repository::key::profile_key_for_target;
use crate::{
    CompilerOptions, HostEnvironment, Module, Package, ProfileEnvironment, ProfileOptions,
    Repository, RepositoryError, Revision, Target,
};

/// One resolved semantic profile.
#[derive(Debug, Clone)]
pub struct Profile {
    /// The canonical profile key.
    pub key: ProfileKey,
    /// The resolved environment values.
    pub env: ProfileEnvironment,
}

impl Profile {
    /// Build one resolved semantic profile from one canonical key.
    pub fn from_key(key: ProfileKey, environment: &HostEnvironment) -> Self {
        let env = ProfileEnvironment::from_key(&key.env, environment, key.debug);

        Self { key, env }
    }

    /// Return the deterministic profile id for this profile.
    pub fn id(&self) -> ProfileId {
        ProfileId::new(self.key.stable_hash())
    }

    /// Return the deterministic profile id for one canonical key.
    pub fn id_for_key(key: &ProfileKey) -> ProfileId {
        ProfileId::new(key.stable_hash())
    }
}

impl Repository {
    /// Get the effective profile for one module.
    pub fn module_profile(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Result<Arc<Profile>, RepositoryError> {
        let revision_state = self.revision(revision)?;
        let Some(module) = self.module(revision, module_id)? else {
            return Err(RepositoryError::MissingModule { module: module_id });
        };
        let Some(package) = self.package(revision, module.package_id)? else {
            return Err(RepositoryError::MissingPackage {
                package: module.package_id,
            });
        };

        let compiler_options = self.module_compiler_options(revision, &package, &module)?;
        let package_options = self.package_options(revision, package.id)?;

        let (target, profile_config) = if let Some(package_options) = package_options.as_ref() {
            let target = self
                .package_default_target(revision, package.id)?
                .map(|(_, target)| target)
                .unwrap_or_default();
            let profile_config = compiler_options
                .profile
                .as_ref()
                .and_then(|name| package_options.profiles.get(name));
            (target, profile_config)
        } else {
            (Target::default(), None)
        };

        let profile = self.profile_from_target(
            &target,
            &compiler_options,
            profile_config,
            &revision_state.host,
        );

        Ok(profile)
    }

    /// Get the effective profile for one package.
    pub fn package_profile(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Arc<Profile>, RepositoryError> {
        let revision_state = self.revision(revision)?;
        let Some(package) = self.package(revision, package_id)? else {
            return Err(RepositoryError::MissingPackage {
                package: package_id,
            });
        };
        let package_options = self.package_options(revision, package.id)?;
        let compiler_options = package_options
            .as_ref()
            .map(|package_options| package_options.compiler.clone())
            .unwrap_or_default();

        let target = self
            .package_default_target(revision, package.id)?
            .map(|(_, target)| target)
            .unwrap_or_default();
        let profile_config = package_options.as_ref().and_then(|package_options| {
            target
                .profile
                .as_ref()
                .or(compiler_options.profile.as_ref())
                .and_then(|name| package_options.profiles.get(name))
        });

        let profile = self.profile_from_target(
            &target,
            &compiler_options,
            profile_config,
            &revision_state.host,
        );

        Ok(profile)
    }

    /// Get the profile selected by one module target.
    pub fn module_target_profile(
        &self,
        revision: Revision,
        module_id: ModuleId,
        target_id: TargetId,
    ) -> Result<Option<Arc<Profile>>, RepositoryError> {
        let revision_state = self.revision(revision)?;
        let Some(module) = self.module(revision, module_id)? else {
            return Err(RepositoryError::MissingModule { module: module_id });
        };
        let Some(package) = self.package(revision, module.package_id)? else {
            return Err(RepositoryError::MissingPackage {
                package: module.package_id,
            });
        };
        if target_id.package_id() != module.package_id {
            return Ok(None);
        }

        let Some(target) = self.effective_target(revision, target_id)? else {
            return Ok(None);
        };

        let compiler_options = self.module_compiler_options(revision, &package, &module)?;
        let package_options = self.package_options(revision, package.id)?;
        let profile_config = package_options.as_ref().and_then(|package_options| {
            target
                .profile
                .as_ref()
                .or(compiler_options.profile.as_ref())
                .and_then(|name| package_options.profiles.get(name))
        });

        let profile = self.profile_from_target(
            &target,
            &compiler_options,
            profile_config,
            &revision_state.host,
        );

        Ok(Some(profile))
    }

    /// Return the exact profiles present in one pinned revision.
    fn profiles(
        &self,
        revision: Revision,
    ) -> Result<Arc<OrdMap<ProfileId, Arc<Profile>>>, RepositoryError> {
        let _revision_state = self.revision(revision)?;
        let revision_cache = self.revision_cache(revision);

        if let Some(profiles) = revision_cache.profiles.get() {
            return Ok(Arc::clone(profiles));
        }

        let mut profiles = OrdMap::new();

        // package profiles
        for package_id in self.package_ids(revision)? {
            let profile = self.package_profile(revision, package_id)?;
            profiles.insert(profile.id(), profile);
        }

        // module profiles
        for module_id in self.module_ids(revision)? {
            let profile = self.module_profile(revision, module_id)?;
            profiles.insert(profile.id(), profile);
        }

        // target profiles
        for module_id in self.module_ids(revision)? {
            let Some(module) = self.module(revision, module_id)? else {
                continue;
            };
            let Some(package) = self.package(revision, module.package_id)? else {
                return Err(RepositoryError::MissingPackage {
                    package: module.package_id,
                });
            };

            for target_id in package.targets.keys() {
                let Some(profile) = self.module_target_profile(revision, module_id, *target_id)?
                else {
                    continue;
                };

                profiles.insert(profile.id(), profile);
            }
        }

        let profiles = Arc::new(profiles);
        let profiles = revision_cache.profiles.get_or_init(|| profiles);

        Ok(Arc::clone(profiles))
    }

    /// Return one exact revision-scoped profile by id when present.
    pub fn profile(
        &self,
        revision: Revision,
        profile_id: ProfileId,
    ) -> Result<Option<Arc<Profile>>, RepositoryError> {
        let profiles = self.profiles(revision)?;

        Ok(profiles.get(&profile_id).cloned())
    }

    /// Return all exact profile ids present in one revision.
    pub fn profile_ids(&self, revision: Revision) -> Result<Vec<ProfileId>, RepositoryError> {
        let profiles = self.profiles(revision)?;

        Ok(profiles.keys().copied().collect())
    }

    /// Build one resolved profile from one canonical key in one revision.
    pub fn profile_from_key(
        &self,
        revision: Revision,
        key: ProfileKey,
    ) -> Result<Profile, RepositoryError> {
        let revision_state = self.revision(revision)?;

        Ok(Profile::from_key(key, &revision_state.host))
    }

    /// Return the exact revision-scoped profile for one module and profile id when present.
    pub fn module_profile_by_id(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Result<Option<Arc<Profile>>, RepositoryError> {
        let default_profile = self.module_profile(revision, module_id)?;
        if default_profile.id() == profile_id {
            return Ok(Some(default_profile));
        }

        let Some(module) = self.module(revision, module_id)? else {
            return Err(RepositoryError::MissingModule { module: module_id });
        };
        let Some(package) = self.package(revision, module.package_id)? else {
            return Err(RepositoryError::MissingPackage {
                package: module.package_id,
            });
        };

        for target_id in package.targets.keys() {
            let Some(profile) = self.module_target_profile(revision, module_id, *target_id)? else {
                continue;
            };

            if profile.id() == profile_id {
                return Ok(Some(profile));
            }
        }

        Ok(None)
    }

    /// Build profile compiler options for one module.
    fn module_compiler_options(
        &self,
        revision: Revision,
        package: &Package,
        _module: &Module,
    ) -> Result<CompilerOptions, RepositoryError> {
        let compiler_options = self
            .package_options(revision, package.id)?
            .map(|package_options| package_options.compiler)
            .unwrap_or_default();

        Ok(compiler_options)
    }

    /// Build one resolved profile from one target and effective option set.
    fn profile_from_target(
        &self,
        target: &Target,
        compiler_options: &CompilerOptions,
        profile_config: Option<&ProfileOptions>,
        environment: &HostEnvironment,
    ) -> Arc<Profile> {
        let key = profile_key_for_target(target, compiler_options, profile_config, environment);

        Arc::new(Profile::from_key(key, environment))
    }
}
