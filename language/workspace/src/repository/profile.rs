use destack_artifact::ProfileKey;
use destack_source::{ModuleId, ProfileId};
use im::OrdMap;
use std::sync::Arc;

use crate::{EnvironmentSnapshot, ProfileEnv, Repository, RepositoryError, Revision};

/// One resolved semantic profile.
#[derive(Debug, Clone)]
pub struct Profile {
    /// The canonical profile key.
    pub key: ProfileKey,
    /// The resolved environment values.
    pub env: ProfileEnv,
}

impl Profile {
    /// Build one resolved semantic profile from one canonical key.
    pub fn from_key(key: ProfileKey, environment: &EnvironmentSnapshot) -> Self {
        let env = ProfileEnv::from_snapshot(&key.env, environment, key.debug);

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
    /// Return the exact profiles present in one pinned revision.
    pub fn profiles(
        &self,
        revision: Revision,
    ) -> Result<Arc<OrdMap<ProfileId, Profile>>, RepositoryError> {
        let revision_state = self.revision(revision)?;
        if let Some(profiles) = revision_state.profiles.get() {
            return Ok(Arc::clone(profiles));
        }

        let workspace = self.workspace(revision)?;
        let mut profiles = OrdMap::new();

        // default profiles
        for (&module_id, _) in workspace.modules().iter() {
            let profile = self.default_profile_for_module(revision, module_id)?;
            profiles.insert(profile.id(), profile);
        }

        // target profiles
        for (&module_id, module) in workspace.modules().iter() {
            let Some(package) = workspace.package(module.package_id) else {
                return Err(RepositoryError::MissingPackage {
                    package: module.package_id,
                });
            };

            for target_id in package.targets.keys() {
                let Some(profile) = self.profile_for_target(revision, module_id, target_id)? else {
                    continue;
                };

                profiles.insert(profile.id(), profile);
            }
        }

        let profiles = Arc::new(profiles);
        let _ = revision_state.profiles.set(Arc::clone(&profiles));

        Ok(revision_state
            .profiles
            .get()
            .map(Arc::clone)
            .unwrap_or(profiles))
    }

    /// Return one exact revision-scoped profile by id when present.
    pub fn profile(
        &self,
        revision: Revision,
        profile_id: ProfileId,
    ) -> Result<Option<Profile>, RepositoryError> {
        let profiles = self.profiles(revision)?;

        Ok(profiles.get(&profile_id).cloned())
    }

    /// Return the exact revision-scoped profile for one module and profile id when present.
    pub fn profile_for_module_id(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Result<Option<Profile>, RepositoryError> {
        let default_profile = self.default_profile_for_module(revision, module_id)?;
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
            let Some(profile) = self.profile_for_target(revision, module_id, target_id)? else {
                continue;
            };

            if profile.id() == profile_id {
                return Ok(Some(profile));
            }
        }

        Ok(None)
    }
}
