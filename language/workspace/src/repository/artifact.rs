use std::collections::HashSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use destack_artifact::{
    ArtifactKey, ArtifactStamp, ArtifactVersion, Ast, DirAnalyzed, DirBase, DirDeclared,
    DirElaborated, DirInterface, DirPatched, DirPrepared, DirResolved, IntrinsicEnvironment,
    LanguageEnvironment, LibraryEnvironment, MirBase, MirOptimized, ModuleGraph, ModuleOutput,
    PackageOutput, ProfileKey,
};
use destack_source::{ModuleId, PackageId, ProfileId, TargetId};

use crate::Target;
use crate::repository::{Repository, RepositoryError, Revision};

impl Repository {
    /// Return one available profile id for one revision-scoped module artifact query.
    pub fn available_profile_id_for_module(
        &self,
        revision: Revision,
        module_id: ModuleId,
        requested_profile_id: ProfileId,
        require_analyzed: bool,
    ) -> Option<ProfileId> {
        if self.has_available_profile_artifacts(
            revision,
            module_id,
            requested_profile_id,
            require_analyzed,
        ) {
            return Some(requested_profile_id);
        }

        let mut profile_ids: Vec<_> = self
            .profile_ids_for_module(revision, module_id)
            .ok()?
            .into_iter()
            .collect();
        profile_ids.sort_unstable();

        profile_ids.into_iter().find(|profile_id| {
            self.has_available_profile_artifacts(revision, module_id, *profile_id, require_analyzed)
        })
    }

    /// Return the profiles that can exist for one revision-scoped module.
    pub fn profile_ids_for_module(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Result<HashSet<ProfileId>, RepositoryError> {
        let Some(module) = self.module(revision, module_id)? else {
            return Ok(HashSet::new());
        };
        let Some(package) = self.package(revision, module.package_id)? else {
            return Ok(HashSet::new());
        };

        let mut profile_ids = HashSet::new();
        profile_ids.insert(self.default_profile_id_for_module(revision, module_id)?);

        for target_id in package.targets.keys() {
            if let Some(profile_id) = self.profile_id_for_target(revision, module_id, target_id)? {
                profile_ids.insert(profile_id);
            }
        }

        Ok(profile_ids)
    }

    /// Return the exact artifact version for one revision-scoped artifact key.
    pub fn artifact_version(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> ArtifactVersion {
        let stamp = self.artifact_stamp(revision, artifact_key);

        ArtifactVersion::new(*artifact_key, stamp)
    }

    /// Return the artifact stamp for one revision-scoped artifact key.
    pub fn artifact_stamp(&self, revision: Revision, artifact_key: &ArtifactKey) -> ArtifactStamp {
        match artifact_key {
            ArtifactKey::ModuleGraph { profile } => {
                let mut module_ids = self.workspace_module_ids(revision).unwrap_or_default();
                module_ids.sort_unstable();
                let module_stamps = module_ids
                    .into_iter()
                    .map(|module_id| {
                        (
                            module_id,
                            self.artifact_stamp(
                                revision,
                                &ArtifactKey::dir_resolved(module_id, *profile),
                            ),
                        )
                    })
                    .collect::<Vec<_>>();
                ArtifactStamp::new(self.hash_artifact_stamp(&(
                    artifact_key,
                    profile,
                    module_stamps,
                )))
            }
            ArtifactKey::LanguageEnvironment { profile }
            | ArtifactKey::IntrinsicEnvironment { profile }
            | ArtifactKey::LibraryEnvironment { profile } => {
                ArtifactStamp::new(self.hash_artifact_stamp(&(artifact_key, profile)))
            }
            ArtifactKey::Ast { module } | ArtifactKey::DirBase { module } => {
                let module = self.module_output_stamp_input(revision, *module);
                ArtifactStamp::new(self.hash_artifact_stamp(&(artifact_key, module)))
            }
            ArtifactKey::DirPrepared { module, profile }
            | ArtifactKey::DirResolved { module, profile }
            | ArtifactKey::DirDeclared { module, profile }
            | ArtifactKey::DirInterface { module, profile }
            | ArtifactKey::DirAnalyzed { module, profile }
            | ArtifactKey::DirElaborated { module, profile }
            | ArtifactKey::DirPatched { module, profile } => {
                let module = self.module_output_stamp_input(revision, *module);
                ArtifactStamp::new(self.hash_artifact_stamp(&(artifact_key, module, profile)))
            }
            ArtifactKey::MirBase {
                module,
                profile,
                target,
            }
            | ArtifactKey::MirOptimized {
                module,
                profile,
                target,
            } => {
                let module_id = *module;
                let module = self.module_output_stamp_input(revision, module_id);
                let target = self.module_target_stamp_input(revision, module_id, *target);

                ArtifactStamp::new(self.hash_artifact_stamp(&(
                    artifact_key,
                    module,
                    profile,
                    target,
                )))
            }
            ArtifactKey::ModuleOutput { module, target } => {
                let module_id = *module;
                let module = self.module_output_stamp_input(revision, module_id);
                let profile = self
                    .profile_for_target(revision, module_id, target)
                    .ok()
                    .flatten()
                    .map(|profile| profile.key);
                let target = self.module_target_stamp_input(revision, module_id, *target);

                ArtifactStamp::new(self.hash_artifact_stamp(&(
                    artifact_key,
                    module,
                    profile,
                    target,
                )))
            }
            ArtifactKey::PackageOutput { package, target } => {
                let package_id = *package;
                let package = self.package_artifact_stamp_input(revision, package_id);
                let target = self.package_target_stamp_input(revision, package_id, *target);

                ArtifactStamp::new(self.hash_artifact_stamp(&(artifact_key, package, target)))
            }
        }
    }

    /// Check whether one module profile has the required live artifacts.
    fn has_available_profile_artifacts(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        require_analyzed: bool,
    ) -> bool {
        let is_resolved = self.dir_resolved(revision, module_id, profile_id).is_some();
        if !is_resolved {
            return false;
        }

        if !require_analyzed {
            return true;
        }

        self.dir_analyzed(revision, module_id, profile_id).is_some()
    }

    /// Return one structural module stamp input for one revision module.
    fn module_output_stamp_input(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Option<impl Hash> {
        let module = self.module(revision, module_id).ok().flatten()?;
        let content_id = self
            .file_content_id(revision, module.file_id)
            .ok()
            .flatten();

        Some((
            module.id,
            module.file_id,
            content_id,
            module.package_id,
            module.tsconfig_file_id,
            module.language_type,
            module.source_type,
            module.module_format,
            module.loader,
            module.source,
        ))
    }

    /// Return one structural package stamp input for one revision package.
    fn package_artifact_stamp_input(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Option<impl Hash> {
        let package = self.package(revision, package_id).ok().flatten()?;
        let package_content_id = package
            .package_file_id
            .and_then(|file_id| self.file_content_id(revision, file_id).ok().flatten());
        let destack_content_id = package
            .destack_file_id
            .and_then(|file_id| self.file_content_id(revision, file_id).ok().flatten());
        let tsconfig_content_id = package
            .tsconfig_file_id
            .and_then(|file_id| self.file_content_id(revision, file_id).ok().flatten());

        Some((
            package.id,
            package.kind,
            package_content_id,
            destack_content_id,
            tsconfig_content_id,
            package.name.clone(),
            package.version.clone(),
        ))
    }

    /// Return one resolved target stamp input for one revision module target.
    fn module_target_stamp_input(
        &self,
        revision: Revision,
        module_id: ModuleId,
        target_id: TargetId,
    ) -> Option<(Target, Option<ProfileKey>)> {
        let module = self.module(revision, module_id).ok().flatten()?;
        let package = self.package(revision, module.package_id).ok().flatten()?;
        let target = package.targets.get(&target_id).cloned().or_else(|| {
            self.target_name_by_target_id(target_id)
                .and_then(|name| Target::implicit_for_name(name.as_ref()))
        })?;
        let profile = self
            .profile_for_target(revision, module_id, &target_id)
            .ok()
            .flatten()
            .map(|profile| profile.key);

        Some((target, profile))
    }

    /// Return one resolved target stamp input for one revision package target.
    fn package_target_stamp_input(
        &self,
        revision: Revision,
        package_id: PackageId,
        target_id: TargetId,
    ) -> Option<Target> {
        let package = self.package(revision, package_id).ok().flatten()?;

        package.targets.get(&target_id).cloned().or_else(|| {
            self.target_name_by_target_id(target_id)
                .and_then(|name| Target::implicit_for_name(name.as_ref()))
        })
    }

    /// Hash one structural tuple into one artifact stamp value.
    fn hash_artifact_stamp(&self, value: &impl Hash) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    /// Return the module graph for one revision-scoped profile.
    pub fn module_graph(
        &self,
        revision: Revision,
        profile_id: ProfileId,
    ) -> Option<Arc<ModuleGraph>> {
        let version = self.artifact_version(revision, &ArtifactKey::module_graph(profile_id));

        self.artifact_store().module_graph(&version)
    }

    /// Return the language environment for one revision-scoped profile.
    pub fn language_environment(
        &self,
        revision: Revision,
        profile_id: ProfileId,
    ) -> Option<Arc<LanguageEnvironment>> {
        let version =
            self.artifact_version(revision, &ArtifactKey::language_environment(profile_id));

        self.artifact_store().language_environment(&version)
    }

    /// Return the intrinsic environment for one revision-scoped profile.
    pub fn intrinsic_environment(
        &self,
        revision: Revision,
        profile_id: ProfileId,
    ) -> Option<Arc<IntrinsicEnvironment>> {
        let version =
            self.artifact_version(revision, &ArtifactKey::intrinsic_environment(profile_id));

        self.artifact_store().intrinsic_environment(&version)
    }

    /// Return the library environment for one revision-scoped profile.
    pub fn library_environment(
        &self,
        revision: Revision,
        profile_id: ProfileId,
    ) -> Option<Arc<LibraryEnvironment>> {
        let version =
            self.artifact_version(revision, &ArtifactKey::library_environment(profile_id));

        self.artifact_store().library_environment(&version)
    }

    /// Return the AST for one revision-scoped module.
    pub fn ast(&self, revision: Revision, module_id: ModuleId) -> Option<Arc<Ast>> {
        let version = self.artifact_version(revision, &ArtifactKey::ast(module_id));

        self.artifact_store().ast(&version)
    }

    /// Return the base DIR for one revision-scoped module.
    pub fn dir_base(&self, revision: Revision, module_id: ModuleId) -> Option<Arc<DirBase>> {
        let version = self.artifact_version(revision, &ArtifactKey::dir_base(module_id));

        self.artifact_store().dir_base(&version)
    }

    /// Return the prepared DIR for one revision-scoped module profile.
    pub fn dir_prepared(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<DirPrepared>> {
        let version =
            self.artifact_version(revision, &ArtifactKey::dir_prepared(module_id, profile_id));

        self.artifact_store().dir_prepared(&version)
    }

    /// Return the resolved DIR for one revision-scoped module profile.
    pub fn dir_resolved(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<DirResolved>> {
        let version =
            self.artifact_version(revision, &ArtifactKey::dir_resolved(module_id, profile_id));

        self.artifact_store().dir_resolved(&version)
    }

    /// Return the declared DIR for one revision-scoped module profile.
    pub fn dir_declared(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<DirDeclared>> {
        let version =
            self.artifact_version(revision, &ArtifactKey::dir_declared(module_id, profile_id));

        self.artifact_store().dir_declared(&version)
    }

    /// Return the interface DIR for one revision-scoped module profile.
    pub fn dir_interface(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<DirInterface>> {
        let version =
            self.artifact_version(revision, &ArtifactKey::dir_interface(module_id, profile_id));

        self.artifact_store().dir_interface(&version)
    }

    /// Return the analyzed DIR for one revision-scoped module profile.
    pub fn dir_analyzed(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<DirAnalyzed>> {
        let version =
            self.artifact_version(revision, &ArtifactKey::dir_analyzed(module_id, profile_id));

        self.artifact_store().dir_analyzed(&version)
    }

    /// Return the elaborated DIR for one revision-scoped module profile.
    pub fn dir_elaborated(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<DirElaborated>> {
        let version = self.artifact_version(
            revision,
            &ArtifactKey::dir_elaborated(module_id, profile_id),
        );

        self.artifact_store().dir_elaborated(&version)
    }

    /// Return the patched DIR for one revision-scoped module profile.
    pub fn dir_patched(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> Option<Arc<DirPatched>> {
        let version =
            self.artifact_version(revision, &ArtifactKey::dir_patched(module_id, profile_id));

        self.artifact_store().dir_patched(&version)
    }

    /// Return the base MIR for one revision-scoped module profile target.
    pub fn mir_base(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        target_id: TargetId,
    ) -> Option<Arc<MirBase>> {
        let version = self.artifact_version(
            revision,
            &ArtifactKey::mir_base(module_id, profile_id, target_id),
        );

        self.artifact_store().mir_base(&version)
    }

    /// Return the optimized MIR for one revision-scoped module profile target.
    pub fn mir_optimized(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        target_id: TargetId,
    ) -> Option<Arc<MirOptimized>> {
        let version = self.artifact_version(
            revision,
            &ArtifactKey::mir_optimized(module_id, profile_id, target_id),
        );

        self.artifact_store().mir_optimized(&version)
    }

    /// Return the generated module output for one revision-scoped target.
    pub fn module_output(
        &self,
        revision: Revision,
        module_id: ModuleId,
        target_id: TargetId,
    ) -> Option<Arc<ModuleOutput>> {
        let version =
            self.artifact_version(revision, &ArtifactKey::module_output(module_id, target_id));

        self.artifact_store().module_output(&version)
    }

    /// Return the package output for one revision-scoped target.
    pub fn package_output(
        &self,
        revision: Revision,
        package_id: PackageId,
        target_id: TargetId,
    ) -> Option<Arc<PackageOutput>> {
        let version = self.artifact_version(
            revision,
            &ArtifactKey::package_output(package_id, target_id),
        );

        self.artifact_store().package_output(&version)
    }
}
