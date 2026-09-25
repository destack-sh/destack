use std::sync::Arc;

use im::OrdMap;
use tspp_artifact::ProfileKey;
use tspp_source::{ModuleId, PackageId, ProfileId, TargetId};

use crate::repository::key::profile_key_for_target;
use crate::{
    CompilerOptions, ConditionSet, Destack, DestackFile, Environment, ProfileOptions, Repository,
    RepositoryError, Revision, Target,
};

/// One resolved semantic profile.
#[derive(Debug, Clone)]
pub struct Profile {
    /// The canonical profile key.
    pub key: ProfileKey,
}

impl Profile {
    /// Build one resolved semantic profile from one canonical key.
    pub fn from_key(key: ProfileKey) -> Self {
        Self { key }
    }

    /// Return the deterministic profile id for this profile.
    pub fn id(&self) -> ProfileId {
        ProfileId::new(self.key.stable_hash())
    }

    /// Return the deterministic profile id for one canonical key.
    pub fn id_for_key(key: &ProfileKey) -> ProfileId {
        ProfileId::new(key.stable_hash())
    }

    /// Return active source graph and runtime conditions.
    pub fn conditions(&self) -> &ConditionSet {
        &self.key.conditions
    }
}

impl Repository {
    /// Build the profile selected by one target.
    pub fn profile_for_target(
        &self,
        revision: Revision,
        target_id: TargetId,
    ) -> Result<Arc<Profile>, RepositoryError> {
        let revision_state = self.revision(revision)?;
        let package_id = target_id.package_id();

        // explicit or built-in target
        let target = self
            .target_or_builtin(revision, target_id)?
            .ok_or(RepositoryError::MissingTarget { target: target_id })?;
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
            config.as_deref(),
            &revision_state.environment,
            product.as_deref(),
            product_role.as_deref(),
        )?;
        Ok(profile)
    }

    /// Build the profile selected by one module target.
    pub fn profile_for_module_target(
        &self,
        revision: Revision,
        module_id: ModuleId,
        target_id: TargetId,
    ) -> Result<Arc<Profile>, RepositoryError> {
        let Some(module) = self.module(revision, module_id)? else {
            return Err(RepositoryError::MissingModule { module: module_id });
        };

        // target must belong to the module package
        if target_id.package_id() != module.package_id {
            return Err(RepositoryError::TargetPackageMismatch {
                module: module_id,
                target: target_id,
            });
        }

        self.profile_for_target(revision, target_id)
    }

    /// Return the exact profiles present in one pinned revision.
    fn profiles(
        &self,
        revision: Revision,
    ) -> Result<Arc<OrdMap<ProfileId, Arc<Profile>>>, RepositoryError> {
        let revision_state = self.revision(revision)?;
        let revision_cache = revision_state.cache();

        if let Some(profiles) = revision_cache.profiles.get() {
            return Ok(profiles.clone());
        }

        let mut profiles = OrdMap::new();

        // explicit and built-in target profiles
        for package_id in self.package_ids(revision)? {
            let target_ids = self.profile_target_ids(revision, package_id)?;

            for target_id in target_ids {
                let profile = self.profile_for_target(revision, target_id)?;

                profiles.insert(profile.id(), profile);
            }
        }

        let profiles = Arc::new(profiles);
        let profiles = revision_cache.profiles.get_or_init(|| profiles);

        Ok(profiles.clone())
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

    /// Return profile ids addressable through targets in one revision.
    pub fn profile_ids_for_targets(
        &self,
        revision: Revision,
    ) -> Result<Vec<ProfileId>, RepositoryError> {
        let profiles = self.profiles(revision)?;

        Ok(profiles.keys().copied().collect())
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
        config: Option<&DestackFile>,
        environment: &Environment,
        product: Option<&str>,
        product_role: Option<&str>,
    ) -> Result<Arc<Profile>, RepositoryError> {
        let destack = config.map(|config| &config.destack);
        let profile_config =
            Self::profile_options_for_target(target, compiler_options, destack, environment);
        let product_config =
            product.and_then(|product| destack.and_then(|config| config.products.get(product)));
        let key = profile_key_for_target(
            target_name,
            target,
            compiler_options,
            profile_config,
            config,
            environment,
            product,
            product_config,
            product_role,
        )?;

        Ok(Arc::new(Profile::from_key(key)))
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
        environment: &'a Environment,
    ) -> Option<&'a ProfileOptions> {
        let config = config?;
        let profile_name = environment.selection.profile.as_ref().or(target
            .conditions
            .profile
            .as_ref()
            .or(compiler_options.profile.as_ref()))?;

        config.profiles.get(profile_name)
    }
}
