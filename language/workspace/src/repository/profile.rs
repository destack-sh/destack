use std::sync::Arc;

use destack_artifact::ProfileKey;
use destack_source::{ModuleId, PackageId, ProfileId, TargetId};
use im::OrdMap;

use crate::repository::key::profile_key_for_target;
use crate::{
    CompilerOptions, ConditionSet, Destack, DestackFile, HostEnvironment, ProfileEnvironment,
    ProfileOptions, Repository, RepositoryError, Revision, Target,
};

/// One resolved semantic profile.
#[derive(Debug, Clone)]
pub struct Profile {
    /// The canonical profile key.
    pub key: ProfileKey,
    /// The active source graph conditions.
    pub conditions: ConditionSet,
    /// The resolved environment values.
    pub env: ProfileEnvironment,
}

impl Profile {
    /// Build one resolved semantic profile from one canonical key.
    pub fn from_key(key: ProfileKey, environment: &HostEnvironment) -> Self {
        let env = ProfileEnvironment::from_key(&key, environment);
        let conditions = conditions_from_key(&key);

        Self {
            key,
            conditions,
            env,
        }
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

/// Build source graph conditions from one canonical profile key.
fn conditions_from_key(key: &ProfileKey) -> ConditionSet {
    ConditionSet {
        modes: key.modes.iter().cloned().collect(),
        roles: key.roles.iter().cloned().collect(),
        features: key.features.iter().cloned().collect(),
        tags: key.tags.iter().cloned().collect(),
        target: key.target.clone(),
        product: key.product.clone(),
        platform: Some(key.platform),
        host: Some(key.host),
        runtime: Some(key.runtime),
    }
}

impl Repository {
    /// Get the profile selected by one explicit target.
    pub fn target_profile(
        &self,
        revision: Revision,
        target_id: TargetId,
    ) -> Result<Option<Arc<Profile>>, RepositoryError> {
        let revision_state = self.revision(revision)?;
        let package_id = target_id.package_id();

        // explicit or built-in target
        let Some(target) = self.target_or_builtin(revision, target_id)? else {
            return Ok(None);
        };
        let target_name = self.target_name(revision, target_id)?;

        // target profile inputs
        let (config, compiler_options) =
            self.package_config_and_compiler_options(revision, package_id)?;
        let product = self.package_default_product(revision, package_id)?;
        let product_role =
            self.product_role_for_target(revision, package_id, product.as_deref(), &target_name)?;

        // profile identity
        let profile = self.profile_from_target(
            &target_name,
            &target,
            &compiler_options,
            config.as_deref().map(|config| &config.destack),
            &revision_state.host,
            product.as_deref(),
            product_role.as_deref(),
        );

        Ok(Some(profile))
    }

    /// Get the profile selected by one explicit module target.
    pub fn module_target_profile(
        &self,
        revision: Revision,
        module_id: ModuleId,
        target_id: TargetId,
    ) -> Result<Option<Arc<Profile>>, RepositoryError> {
        let Some(module) = self.module(revision, module_id)? else {
            return Err(RepositoryError::MissingModule { module: module_id });
        };

        // target must belong to the module package
        if target_id.package_id() != module.package_id {
            return Ok(None);
        }

        self.target_profile(revision, target_id)
    }

    /// Return the exact profiles present in one pinned revision.
    fn profiles(
        &self,
        revision: Revision,
    ) -> Result<Arc<OrdMap<ProfileId, Arc<Profile>>>, RepositoryError> {
        let revision_state = self.revision(revision)?;
        let revision_cache = revision_state.cache();

        if let Some(profiles) = revision_cache.profiles.get() {
            return Ok(Arc::clone(profiles));
        }

        let mut profiles = OrdMap::new();

        // explicit and built-in target profiles
        for package_id in self.package_ids(revision)? {
            let target_ids = self.profile_target_ids(revision, package_id)?;

            for target_id in target_ids {
                let Some(profile) = self.target_profile(revision, target_id)? else {
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

    /// Return target profile ids addressable in one revision.
    pub fn target_profile_ids(
        &self,
        revision: Revision,
    ) -> Result<Vec<ProfileId>, RepositoryError> {
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
        let Some(module) = self.module(revision, module_id)? else {
            return Err(RepositoryError::MissingModule { module: module_id });
        };
        let target_ids = self.profile_target_ids(revision, module.package_id)?;

        for target_id in target_ids {
            let Some(profile) = self.target_profile(revision, target_id)? else {
                continue;
            };

            if profile.id() == profile_id {
                return Ok(Some(profile));
            }
        }

        Ok(None)
    }

    /// Return target ids with addressable profile keys for one package.
    fn profile_target_ids(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Vec<TargetId>, RepositoryError> {
        let Some(package) = self.package(revision, package_id)? else {
            return Err(RepositoryError::MissingPackage {
                package: package_id,
            });
        };
        let mut target_ids = package.targets.keys().copied().collect::<Vec<_>>();

        // built-in target names are explicit profile addresses
        for target_name in Target::builtin_target_names() {
            let target_id = TargetId::new(package_id, target_name);

            if !target_ids.contains(&target_id) {
                target_ids.push(target_id);
            }
        }

        Ok(target_ids)
    }

    /// Return package config and compiler options.
    fn package_config_and_compiler_options(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<(Option<Arc<DestackFile>>, CompilerOptions), RepositoryError> {
        let config = self.destack_for_package_id(revision, package_id)?;
        let compiler_options = config
            .as_ref()
            .map(|config| config.compiler.clone())
            .unwrap_or_default();

        Ok((config, compiler_options))
    }

    /// Build one resolved profile from one target and effective option set.
    fn profile_from_target(
        &self,
        target_name: &str,
        target: &Target,
        compiler_options: &CompilerOptions,
        config: Option<&Destack>,
        environment: &HostEnvironment,
        product: Option<&str>,
        product_role: Option<&str>,
    ) -> Arc<Profile> {
        let profile_config = Self::profile_options_for_target(target, compiler_options, config);
        let key = profile_key_for_target(
            target_name,
            target,
            compiler_options,
            profile_config,
            config,
            environment,
            product,
            product_role,
        );

        Arc::new(Profile::from_key(key, environment))
    }

    /// Return the active product role when the target belongs to the selected product.
    fn product_role_for_target(
        &self,
        revision: Revision,
        package_id: PackageId,
        product: Option<&str>,
        target: &str,
    ) -> Result<Option<String>, RepositoryError> {
        let Some(product) = product else {
            return Ok(None);
        };

        self.package_product_role_for_target(revision, package_id, product, target)
    }

    /// Return profile options selected by one target and compiler option set.
    fn profile_options_for_target<'a>(
        target: &'a Target,
        compiler_options: &'a CompilerOptions,
        config: Option<&'a Destack>,
    ) -> Option<&'a ProfileOptions> {
        let config = config?;
        let profile_name = target
            .profile
            .as_ref()
            .or(compiler_options.profile.as_ref())?;

        config.profiles.get(profile_name)
    }
}
