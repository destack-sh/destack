use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

use destack_artifact::{
    ArtifactKey, ArtifactStamp, ArtifactVersion, Ast, DirAnalyzed, DirBase, DirDeclared,
    DirElaborated, DirInterface, DirPatched, DirPrepared, DirResolved, IntrinsicEnvironment,
    LanguageEnvironment, LibraryEnvironment, Loader, MirBase, MirOptimized, ModuleGraph,
    ModuleOutput, PackageOutput, ProfileKey,
};
use destack_source::{
    DiagnosticCollection, FileId, LanguageType, ModuleId, PackageId, ProfileId, TargetId,
};
use rustc_hash::FxHasher;

use crate::repository::{FileContentId, Repository, RepositoryError, Revision};
use crate::{ModuleSource, PackageKind, Target};

/// One structural stamp for one revision-scoped source module.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ModuleSourceStamp {
    /// The stable module id.
    module_id: ModuleId,
    /// The backing source file id.
    file_id: FileId,
    /// The current source content id.
    content_id: Option<FileContentId>,
    /// The owning package id.
    package_id: PackageId,
    /// The module language.
    language_type: LanguageType,
    /// The loader strategy.
    loader: Loader,
    /// The module source kind.
    source: ModuleSource,
}

/// One structural stamp for one revision-scoped module output family.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ModuleOutputStamp {
    /// The source module stamp.
    source: ModuleSourceStamp,
    /// The effective tsconfig file id.
    tsconfig_file_id: Option<FileId>,
    /// The current package declaration content id.
    package_file_content_id: Option<FileContentId>,
    /// The current workspace declaration content id.
    destack_file_content_id: Option<FileContentId>,
    /// The current tsconfig content id.
    tsconfig_content_id: Option<FileContentId>,
}

/// One structural stamp for one revision-scoped package output family.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct PackageOutputStamp {
    /// The stable package id.
    package_id: PackageId,
    /// The discovered package kind.
    kind: PackageKind,
    /// The current package declaration content id.
    package_content_id: Option<FileContentId>,
    /// The current workspace declaration content id.
    destack_content_id: Option<FileContentId>,
    /// The current package tsconfig content id.
    tsconfig_content_id: Option<FileContentId>,
    /// The discovered package path.
    path: Option<PathBuf>,
}

/// One structural stamp for one revision-scoped module target selection.
#[derive(Debug, Clone, Hash)]
struct ModuleTargetStamp {
    /// The resolved target.
    target: Target,
    /// The resolved profile key.
    profile: Option<ProfileKey>,
}

/// One structural stamp for one revision-scoped package target selection.
#[derive(Debug, Clone, Hash)]
struct PackageTargetStamp {
    /// The resolved target.
    target: Target,
}

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

    /// Return diagnostics for one exact revision scoped artifact key.
    pub fn artifact_diagnostics(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
    ) -> DiagnosticCollection {
        let version = self.artifact_version(revision, artifact_key);

        self.artifact_store()
            .diagnostics(&version)
            .map(|diagnostics| diagnostics.as_ref().clone())
            .unwrap_or_default()
    }

    /// Return diagnostics across the current artifact family slice for one module profile.
    pub fn module_artifact_diagnostics(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> DiagnosticCollection {
        let mut diagnostics = DiagnosticCollection::new();
        let artifact_keys = [
            ArtifactKey::ast(module_id),
            ArtifactKey::data(module_id),
            ArtifactKey::dir_base(module_id),
            ArtifactKey::dir_prepared(module_id, profile_id),
            ArtifactKey::dir_resolved(module_id, profile_id),
            ArtifactKey::dir_declared(module_id, profile_id),
            ArtifactKey::dir_interface(module_id, profile_id),
            ArtifactKey::dir_analyzed(module_id, profile_id),
            ArtifactKey::dir_elaborated(module_id, profile_id),
            ArtifactKey::dir_patched(module_id, profile_id),
        ];

        for artifact_key in artifact_keys {
            diagnostics.merge_from(&self.artifact_diagnostics(revision, &artifact_key));
        }

        diagnostics
    }

    /// Return diagnostics across the current target artifact family slice for one module target.
    pub fn module_target_artifact_diagnostics(
        &self,
        revision: Revision,
        module_id: ModuleId,
        profile_id: ProfileId,
        target_id: TargetId,
    ) -> DiagnosticCollection {
        let mut diagnostics = self.module_artifact_diagnostics(revision, module_id, profile_id);
        let artifact_keys = [
            ArtifactKey::mir_base(module_id, profile_id, target_id),
            ArtifactKey::mir_optimized(module_id, profile_id, target_id),
            ArtifactKey::module_output(module_id, target_id),
        ];

        for artifact_key in artifact_keys {
            diagnostics.merge_from(&self.artifact_diagnostics(revision, &artifact_key));
        }

        diagnostics
    }

    /// Return the artifact stamp for one revision-scoped artifact key.
    pub fn artifact_stamp(&self, revision: Revision, artifact_key: &ArtifactKey) -> ArtifactStamp {
        let revision_state = self
            .revision(revision)
            .unwrap_or_else(|error| panic!("missing revision state for artifact stamp: {error}"));
        let artifact_stamps = revision_state
            .artifact_stamps
            .get_or_init(|| Arc::new(parking_lot::RwLock::new(rustc_hash::FxHashMap::default())));

        {
            let artifact_stamps = artifact_stamps.read();
            if let Some(stamp) = artifact_stamps.get(artifact_key) {
                return *stamp;
            }
        }

        let stamp = match artifact_key {
            ArtifactKey::ModuleGraph { profile } => {
                self.module_graph_artifact_stamp(revision, artifact_key, *profile)
            }
            ArtifactKey::LanguageEnvironment { profile }
            | ArtifactKey::IntrinsicEnvironment { profile }
            | ArtifactKey::LibraryEnvironment { profile } => {
                self.profile_artifact_stamp(artifact_key, *profile)
            }
            ArtifactKey::Ast { module }
            | ArtifactKey::Data { module }
            | ArtifactKey::DirBase { module } => {
                self.module_source_artifact_stamp(revision, artifact_key, *module)
            }
            ArtifactKey::DirPrepared { module, profile }
            | ArtifactKey::DirResolved { module, profile }
            | ArtifactKey::DirDeclared { module, profile }
            | ArtifactKey::DirInterface { module, profile }
            | ArtifactKey::DirAnalyzed { module, profile }
            | ArtifactKey::DirElaborated { module, profile }
            | ArtifactKey::DirPatched { module, profile } => {
                self.module_profile_artifact_stamp(revision, artifact_key, *module, *profile)
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
            } => self.module_target_profile_artifact_stamp(
                revision,
                artifact_key,
                *module,
                *profile,
                *target,
            ),
            ArtifactKey::ModuleOutput { module, target } => {
                self.module_output_artifact_stamp(revision, artifact_key, *module, *target)
            }
            ArtifactKey::PackageOutput { package, target } => {
                self.package_output_artifact_stamp(revision, artifact_key, *package, *target)
            }
        };

        let mut artifact_stamps = artifact_stamps.write();
        let entry = artifact_stamps.entry(*artifact_key).or_insert(stamp);

        *entry
    }

    // FUGU #Architecture: revisit workspace artifact stamp business

    /// Return the stamp for one module graph artifact.
    fn module_graph_artifact_stamp(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
        profile_id: ProfileId,
    ) -> ArtifactStamp {
        let mut module_ids = self.workspace_module_ids(revision).unwrap_or_default();
        module_ids.sort_unstable();

        let module_stamps = module_ids
            .into_iter()
            .map(|module_id| {
                (
                    module_id,
                    self.artifact_stamp(
                        revision,
                        &ArtifactKey::dir_resolved(module_id, profile_id),
                    ),
                )
            })
            .collect::<Vec<_>>();

        self.stamp_for(&(artifact_key, profile_id, module_stamps))
    }

    /// Return the stamp for one profile-scoped artifact family.
    fn profile_artifact_stamp(
        &self,
        artifact_key: &ArtifactKey,
        profile_id: ProfileId,
    ) -> ArtifactStamp {
        self.stamp_for(&(artifact_key, profile_id))
    }

    /// Return the stamp for one source-scoped module artifact family.
    fn module_source_artifact_stamp(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
        module_id: ModuleId,
    ) -> ArtifactStamp {
        let module = self.module_source_stamp(revision, module_id);

        self.stamp_for(&(artifact_key, module))
    }

    /// Return the stamp for one module/profile artifact family.
    fn module_profile_artifact_stamp(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
        module_id: ModuleId,
        profile_id: ProfileId,
    ) -> ArtifactStamp {
        let module = self.module_output_stamp(revision, module_id);

        self.stamp_for(&(artifact_key, module, profile_id))
    }

    /// Return the stamp for one module/profile/target artifact family.
    fn module_target_profile_artifact_stamp(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
        module_id: ModuleId,
        profile_id: ProfileId,
        target_id: TargetId,
    ) -> ArtifactStamp {
        let module = self.module_output_stamp(revision, module_id);
        let target = self.module_target_stamp(revision, module_id, target_id);

        self.stamp_for(&(artifact_key, module, profile_id, target))
    }

    /// Return the stamp for one emitted module artifact.
    fn module_output_artifact_stamp(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
        module_id: ModuleId,
        target_id: TargetId,
    ) -> ArtifactStamp {
        let module = self.module_output_stamp(revision, module_id);
        let profile = self
            .profile_for_target(revision, module_id, &target_id)
            .ok()
            .flatten()
            .map(|profile| profile.key);
        let target = self.module_target_stamp(revision, module_id, target_id);

        self.stamp_for(&(artifact_key, module, profile, target))
    }

    /// Return the stamp for one emitted package artifact.
    fn package_output_artifact_stamp(
        &self,
        revision: Revision,
        artifact_key: &ArtifactKey,
        package_id: PackageId,
        target_id: TargetId,
    ) -> ArtifactStamp {
        let package = self.package_output_stamp(revision, package_id);
        let target = self.package_target_stamp(revision, package_id, target_id);

        self.stamp_for(&(artifact_key, package, target))
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

    /// Return one structural module stamp for one revision module.
    fn module_source_stamp(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Option<ModuleSourceStamp> {
        let revision_state = self.revision(revision).ok()?;
        let workspace = self.workspace(revision).ok()?;
        let module = workspace.module(module_id)?;
        let content_id = revision_state.file_content_id(module.file_id);

        Some(ModuleSourceStamp {
            module_id: module.id,
            file_id: module.file_id,
            content_id,
            package_id: module.package_id,
            language_type: module.language_type,
            loader: module.loader,
            source: module.source,
        })
    }

    /// Return one structural module output stamp for one revision module.
    fn module_output_stamp(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Option<ModuleOutputStamp> {
        let revision_state = self.revision(revision).ok()?;
        let workspace = self.workspace(revision).ok()?;
        let module = workspace.module(module_id)?;
        let package = workspace.package(module.package_id);
        let package_path = package.and_then(|package| package.path.as_ref());
        let package_file_id =
            package_path.map(|path| self.file_id_for_workspace_path(&path.join("package.json")));
        let package_file_id =
            package_file_id.filter(|file_id| revision_state.file_content_id(*file_id).is_some());
        let destack_file_id =
            package_path.map(|path| self.file_id_for_workspace_path(&path.join("destack.json")));
        let destack_file_id =
            destack_file_id.filter(|file_id| revision_state.file_content_id(*file_id).is_some());
        let tsconfig_file_id = module
            .path
            .as_ref()
            .and_then(|path| {
                self.applicable_tsconfig_file_id_at_path(revision, path)
                    .ok()
            })
            .flatten();
        let package_file_content_id =
            package_file_id.and_then(|file_id| revision_state.file_content_id(file_id));
        let destack_file_content_id =
            destack_file_id.and_then(|file_id| revision_state.file_content_id(file_id));
        let tsconfig_content_id =
            tsconfig_file_id.and_then(|file_id| revision_state.file_content_id(file_id));

        let source = self.module_source_stamp(revision, module_id)?;

        Some(ModuleOutputStamp {
            source,
            tsconfig_file_id,
            package_file_content_id,
            destack_file_content_id,
            tsconfig_content_id,
        })
    }

    /// Return one structural package output stamp for one revision package.
    fn package_output_stamp(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Option<PackageOutputStamp> {
        let revision_state = self.revision(revision).ok()?;
        let workspace = self.workspace(revision).ok()?;
        let package = workspace.package(package_id)?;
        let package_file_id = package
            .path
            .as_ref()
            .map(|path| self.file_id_for_workspace_path(&path.join("package.json")));
        let package_file_id =
            package_file_id.filter(|file_id| revision_state.file_content_id(*file_id).is_some());
        let destack_file_id = package
            .path
            .as_ref()
            .map(|path| self.file_id_for_workspace_path(&path.join("destack.json")));
        let destack_file_id =
            destack_file_id.filter(|file_id| revision_state.file_content_id(*file_id).is_some());
        let tsconfig_file_id = package
            .path
            .as_ref()
            .map(|path| self.file_id_for_workspace_path(&path.join("tsconfig.json")));
        let tsconfig_file_id =
            tsconfig_file_id.filter(|file_id| revision_state.file_content_id(*file_id).is_some());
        let package_content_id =
            package_file_id.and_then(|file_id| revision_state.file_content_id(file_id));
        let destack_content_id =
            destack_file_id.and_then(|file_id| revision_state.file_content_id(file_id));
        let tsconfig_content_id =
            tsconfig_file_id.and_then(|file_id| revision_state.file_content_id(file_id));

        Some(PackageOutputStamp {
            package_id: package.id,
            kind: package.kind,
            package_content_id,
            destack_content_id,
            tsconfig_content_id,
            path: package.path.clone(),
        })
    }

    /// Return one resolved module target stamp for one revision target.
    fn module_target_stamp(
        &self,
        revision: Revision,
        module_id: ModuleId,
        target_id: TargetId,
    ) -> Option<ModuleTargetStamp> {
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

        Some(ModuleTargetStamp { target, profile })
    }

    /// Return one resolved package target stamp for one revision target.
    fn package_target_stamp(
        &self,
        revision: Revision,
        package_id: PackageId,
        target_id: TargetId,
    ) -> Option<PackageTargetStamp> {
        let package = self.package(revision, package_id).ok().flatten()?;

        let target = package.targets.get(&target_id).cloned().or_else(|| {
            self.target_name_by_target_id(target_id)
                .and_then(|name| Target::implicit_for_name(name.as_ref()))
        })?;

        Some(PackageTargetStamp { target })
    }

    /// Hash one structural tuple into one artifact stamp value.
    fn hash_artifact_stamp(&self, value: &impl Hash) -> u64 {
        let mut hasher = FxHasher::default();
        value.hash(&mut hasher);
        hasher.finish()
    }

    /// Build one artifact stamp from one structural value.
    fn stamp_for(&self, value: &impl Hash) -> ArtifactStamp {
        ArtifactStamp::new(self.hash_artifact_stamp(value))
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
