use std::collections::BTreeMap;
use std::sync::Arc;

use destack_artifact::{
    ArtifactDependency, ArtifactDependencySet, ArtifactKey, ArtifactOutcome, ArtifactPayload,
    ArtifactProjection, ArtifactProjectionFingerprint, ArtifactVersion, ModuleIndex,
    ModuleIndexProjection,
};
use destack_repository::{
    ArtifactReader, ProviderContext, ProviderError, ProviderResult, Repository, Revision,
};
use destack_source::{ModuleId, ProfileId};

use crate::provide_module_query_context;

use super::module::ModuleIndexer;
use super::program::{ProgramChanges, ProgramIndexer, ProgramModule};

/// Provider that builds query index artifacts for one repository.
#[derive(Debug, Clone)]
pub struct Indexer {
    /// The repository being indexed.
    repository: Arc<Repository>,
}

impl Indexer {
    /// Create an index provider for one repository.
    pub fn new(repository: Arc<Repository>) -> Self {
        Self { repository }
    }

    /// Return the repository being indexed.
    pub(crate) fn repository(&self) -> &Repository {
        &self.repository
    }
}

/// One loaded module index artifact.
struct ModuleIndexArtifact {
    /// The indexed module id.
    module_id: ModuleId,
    /// The current module index payload.
    index: Arc<ModuleIndex>,
    /// The current module index artifact version.
    version: ArtifactVersion,
}

impl Indexer {
    /// Collect the dependency closure for one index artifact.
    pub fn collect(&self, context: &dyn ProviderContext) -> ProviderResult<ArtifactDependencySet> {
        match context.artifact_key() {
            ArtifactKey::ModuleIndex { module, profile } => {
                self.collect_module_index(context, module, profile)
            }
            ArtifactKey::ProgramIndex { profile } => self.collect_program_index(context, profile),
            artifact_key => Err(ProviderError::internal(format!(
                "non index artifact key reached index provider: {artifact_key:?}"
            ))
            .into()),
        }
    }

    /// Collect dependencies for one module index artifact.
    fn collect_module_index(
        &self,
        context: &dyn ProviderContext,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ProviderResult<ArtifactDependencySet> {
        let revision = context.revision();
        let mut dependencies = ArtifactDependencySet::default();

        // index one module from its own checked DIR artifacts
        if !self.module_has_profile(revision, module_id, profile_id)? {
            return Err(ProviderError::internal(format!(
                "module profile not found for module {module_id:?} and profile {profile_id:?}"
            ))
            .into());
        }

        dependencies.require(ArtifactKey::global_environment(profile_id));
        dependencies.require(ArtifactKey::dir_parsed(module_id));
        dependencies.require(ArtifactKey::dir_bound(module_id, profile_id));
        dependencies.require(ArtifactKey::dir_imported(module_id, profile_id));
        dependencies.require(ArtifactKey::dir_expanded(module_id, profile_id));
        dependencies.require(ArtifactKey::dir_exported(module_id, profile_id));
        dependencies.require(ArtifactKey::dir_checked(module_id, profile_id));

        Ok(dependencies)
    }

    /// Collect dependencies for one program index artifact.
    fn collect_program_index(
        &self,
        context: &dyn ProviderContext,
        profile_id: ProfileId,
    ) -> ProviderResult<ArtifactDependencySet> {
        let revision = context.revision();
        let module_ids = self.repository().module_ids(revision).map_err(|error| {
            ProviderError::internal(format!("failed to read repository modules: {error}"))
        })?;

        let mut dependencies = ArtifactDependencySet::default();
        let mut modules = Vec::new();

        // depend on each observable section from each module index in this profile
        for module_id in module_ids {
            if !self.module_has_profile(revision, module_id, profile_id)? {
                continue;
            }

            modules.push(module_id);

            let key = ArtifactKey::module_index(module_id, profile_id);
            for projection in ModuleIndexProjection::ALL {
                dependencies.project(key, projection);
            }
        }

        // derive from the previous payload only when module ordinals still match
        if let Some(base) = context.artifact_base() {
            let previous = self
                .repository()
                .artifact_table()
                .program_index(&base.version);

            if previous.is_some_and(|previous| previous.modules == modules) {
                dependencies.derive_from(base.version);
            }
        }

        Ok(dependencies)
    }

    /// Provide one index artifact.
    pub fn provide(&self, context: &dyn ProviderContext) -> ProviderResult<ArtifactPayload> {
        match context.artifact_key() {
            ArtifactKey::ModuleIndex { module, profile } => {
                self.provide_module_index(context, module, profile)
            }
            ArtifactKey::ProgramIndex { profile } => self.provide_program_index(context, profile),
            artifact_key => Err(ProviderError::internal(format!(
                "non index artifact key reached index provider: {artifact_key:?}"
            ))
            .into()),
        }
    }

    /// Provide one module index artifact.
    fn provide_module_index(
        &self,
        context: &dyn ProviderContext,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ProviderResult<ArtifactPayload> {
        let revision = context.revision();
        let artifacts = ArtifactReader::new(self.repository(), revision);

        // build a module query context from the requested artifacts
        let module = provide_module_query_context(
            self.repository(),
            revision,
            module_id,
            profile_id,
            &artifacts,
        )?;

        // build the checked DIR module index
        let indexer = ModuleIndexer { module: &module };
        let payload = indexer.build();

        Ok(ArtifactPayload::ModuleIndex(Arc::new(payload)))
    }

    /// Provide one program index artifact.
    fn provide_program_index(
        &self,
        context: &dyn ProviderContext,
        profile_id: ProfileId,
    ) -> ProviderResult<ArtifactPayload> {
        let repository = self.repository();
        let revision = context.revision();
        let module_ids = repository.module_ids(revision).map_err(|error| {
            ProviderError::internal(format!("failed to read repository modules: {error}"))
        })?;

        let mut modules = Vec::new();

        // load every ready module index in this profile
        for module_id in module_ids {
            if !self.module_has_profile(revision, module_id, profile_id)? {
                continue;
            }

            let key = ArtifactKey::module_index(module_id, profile_id);
            let version = self.ready_artifact_version(revision, key)?;
            let index = repository
                .artifact_table()
                .module_index(&version)
                .ok_or_else(|| {
                    ProviderError::internal(format!("missing module index payload: {version:?}"))
                })?;

            modules.push(ModuleIndexArtifact {
                module_id,
                index,
                version,
            });
        }

        // reuse the previous program index only when module order is stable
        let previous = context
            .artifact_base()
            .and_then(|base| repository.artifact_table().program_index(&base.version));
        let current_modules = modules
            .iter()
            .map(|artifact| artifact.module_id)
            .collect::<Vec<_>>();
        let previous = previous
            .as_deref()
            .filter(|previous| previous.modules == current_modules);
        let changes = if previous.is_some() {
            self.program_index_changes(context, profile_id, &modules)?
        } else {
            ProgramChanges::all()
        };

        // expose current module indexes to the program indexer
        let modules = modules
            .iter()
            .map(|artifact| ProgramModule {
                module_id: artifact.module_id,
                index: artifact.index.as_ref(),
            })
            .collect();

        // build the incremental program postings index
        let indexer = ProgramIndexer {
            previous,
            modules,
            changes,
        };
        let payload = indexer.build();

        Ok(ArtifactPayload::ProgramIndex(Arc::new(payload)))
    }

    /// Return whether one module has the requested profile in this revision.
    fn module_has_profile(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Result<bool, Box<ProviderError>> {
        let profile = self
            .repository()
            .module_profile_by_id(revision, module_id, profile_id)
            .map_err(|error| {
                ProviderError::internal(format!(
                    "failed to read module profile {module_id:?}/{profile_id:?}: {error}"
                ))
            })?;

        Ok(profile.is_some())
    }

    /// Return the ready artifact version for one key.
    fn ready_artifact_version(
        &self,
        revision: Revision,
        key: ArtifactKey,
    ) -> ProviderResult<ArtifactVersion> {
        let repository = self.repository();
        let version = repository
            .artifact_binding(revision, &key)
            .map_err(|error| {
                ProviderError::internal(format!("failed to read artifact binding {key:?}: {error}"))
            })?
            .filter(|version| {
                matches!(
                    repository.artifact_table().outcome(version),
                    Some(ArtifactOutcome::Ok)
                )
            })
            .ok_or_else(|| {
                ProviderError::internal(format!("program index dependency not built: {key:?}"))
            })?;

        Ok(version)
    }

    /// Return the program index sections whose module projections changed.
    fn program_index_changes(
        &self,
        context: &dyn ProviderContext,
        profile_id: ProfileId,
        modules: &[ModuleIndexArtifact],
    ) -> ProviderResult<ProgramChanges> {
        let base = context
            .artifact_base()
            .ok_or_else(|| ProviderError::internal("missing reusable program index base"))?;
        let dependencies = self
            .repository()
            .artifact_table()
            .dependencies(&base.version)
            .ok_or_else(|| {
                ProviderError::internal(format!(
                    "missing reusable program index dependencies: {:?}",
                    base.version
                ))
            })?;

        let previous_fingerprints = Self::projection_fingerprints(&dependencies);
        let mut changes = ProgramChanges::default();
        for artifact in modules {
            for projection in ModuleIndexProjection::ALL {
                let is_changed = self.projection_changed(
                    &previous_fingerprints,
                    artifact.module_id,
                    profile_id,
                    artifact.version,
                    projection,
                )?;

                changes.mark(projection, is_changed);
            }
        }

        Ok(changes)
    }

    /// Return previous projection fingerprints by projection identity.
    fn projection_fingerprints(
        dependencies: &[ArtifactDependency],
    ) -> BTreeMap<ArtifactProjection, ArtifactProjectionFingerprint> {
        let mut fingerprints = BTreeMap::new();

        for dependency in dependencies {
            if let ArtifactDependency::Projection(dependency) = dependency {
                fingerprints.insert(dependency.projection, dependency.fingerprint);
            }
        }

        fingerprints
    }

    /// Return whether one module index projection changed since the previous program index.
    fn projection_changed(
        &self,
        previous_fingerprints: &BTreeMap<ArtifactProjection, ArtifactProjectionFingerprint>,
        module_id: ModuleId,
        profile_id: ProfileId,
        version: ArtifactVersion,
        projection: ModuleIndexProjection,
    ) -> ProviderResult<bool> {
        let artifact = ArtifactKey::module_index(module_id, profile_id);
        let projection = ArtifactProjection::new(artifact, projection);
        let current = self
            .repository()
            .artifact_table()
            .projection_fingerprint(&version, &projection)
            .ok_or_else(|| {
                ProviderError::internal(format!(
                    "module index projection does not match payload: {projection:?}"
                ))
            })?;
        let previous = previous_fingerprints.get(&projection);

        Ok(previous != Some(&current))
    }
}
