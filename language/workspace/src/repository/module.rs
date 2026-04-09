use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::{ArtifactKey, Loader};
use destack_source::{
    File, FileContent, FileContentEntry, FileId, FileMetadata, FileType, LanguageType, ModuleId,
    PackageId, PathExt, ProfileId, Uri,
};
use im::OrdMap;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::repository::{
    BUILTIN_PACKAGE_ID, FileContentId, FileOrigin, Repository, RepositoryError, Revision, SourceMap,
};
use crate::{
    Module, ModuleDetection, ModuleFormat, ModuleSource, Package, SourceType, TsConfigDeclaration,
    TsConfigOptions,
};

impl Repository {
    /// Complete one module snapshot from one module identity.
    fn complete_module_snapshot(
        &self,
        revision: Revision,
        module: &Module,
        packages: &OrdMap<PackageId, Package>,
    ) -> Result<Module, RepositoryError> {
        let mut module = module.clone();
        let package = packages
            .get(&module.package_id)
            .ok_or(RepositoryError::MissingPackage {
                package: module.package_id,
            })?;
        let tsconfig_file_id =
            self.module_tsconfig_file_id_for_path(revision, module.path.as_deref())?;
        let source_type = self.detect_module_source_type_for_package(
            revision,
            module.path.as_deref(),
            package,
            tsconfig_file_id,
            false,
        )?;
        let module_format = self.detect_module_format_for_package(
            revision,
            module.path.as_deref(),
            module.language_type,
            source_type,
            package,
            tsconfig_file_id,
        )?;

        module.tsconfig_file_id = tsconfig_file_id;
        module.source_type = source_type;
        module.module_format = module_format;

        Ok(module)
    }

    /// Return the synthetic root module id.
    pub fn synthetic_root_module_id(&self) -> ModuleId {
        ModuleId::from_relative_path(PackageId::EPHEMERAL, Path::new("root"))
    }

    /// Return whether one module id is the synthetic root module.
    pub fn is_synthetic_root_module(&self, module_id: ModuleId) -> bool {
        module_id == self.synthetic_root_module_id()
    }

    /// Build module snapshots from one source map.
    pub(crate) fn module_snapshots_from_source_map(
        &self,
        revision: Revision,
        source: &SourceMap,
        packages: &OrdMap<PackageId, Package>,
    ) -> Result<OrdMap<ModuleId, Module>, RepositoryError> {
        let mut modules = OrdMap::new();
        let package_paths = self.package_paths(packages);

        for (file_id, entry) in source.iter() {
            if let Some(module) = self.synthetic_module(*file_id, &entry.origin) {
                modules.insert(module.id, module);
                continue;
            }

            if let Some(module) = self.builtin_module(*file_id, &entry.origin) {
                modules.insert(module.id, module);
                continue;
            }

            let Some(path) = self.path_for_origin(&entry.origin) else {
                continue;
            };
            let file_type = FileType::from_path_or_unknown(&path);
            if !self.is_module_file(&path, file_type) {
                continue;
            }

            let Some(package) = self.package_for_indexed_path(packages, &package_paths, &path)
            else {
                continue;
            };

            let language_type = LanguageType::from(file_type);
            let loader = Loader::from_file_type(file_type);
            let module_id =
                ModuleId::from_path_with_loader(package.id, &path, package.path.as_deref(), None);
            let module = Module::blank(
                module_id,
                *file_id,
                Uri::from_path(&path),
                Some(path),
                package.id,
                language_type,
                loader,
                ModuleSource::User,
            );

            modules.insert(module_id, module);
        }

        let mut module_snapshots = OrdMap::new();

        for module in modules.values() {
            let module = self.complete_module_snapshot(revision, module, packages)?;
            module_snapshots.insert(module.id, module);
        }

        Ok(module_snapshots)
    }

    /// Return one source file snapshot for one revision and file id.
    pub fn file(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<Arc<File>>, RepositoryError> {
        let revision_id = revision;
        let revision = self.revision(revision_id)?;
        let Some(entry) = revision.file_entry(file_id) else {
            return Ok(None);
        };
        let content = self.file_content_by_id(entry.content_id)?;
        let file = self.build_file_at_revision(file_id, &entry.origin, content);

        Ok(Some(Arc::new(file)))
    }

    /// Return one workspace file snapshot for one revision and workspace path.
    pub fn file_for_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<Arc<File>>, RepositoryError> {
        let file_id = self.file_id_for_workspace_path(path);
        self.file(revision, file_id)
    }

    /// Return one workspace path metadata snapshot for one revision and workspace path.
    pub fn metadata_for_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<FileMetadata>, RepositoryError> {
        // file metadata
        if let Some(file) = self.file_for_path(revision, path)? {
            return Ok(Some(FileMetadata::new(
                true,
                false,
                false,
                file.len as u64,
                None,
            )));
        }

        let revision_state = self.revision(revision)?;
        let normalized_path = path.normalize();
        let directory_paths = revision_state
            .directory_paths
            .get_or_init(|| Arc::new(self.directory_paths_from_source_map(&revision_state.source)));

        // directory metadata
        if directory_paths.contains(&normalized_path) {
            return Ok(Some(FileMetadata::new(false, true, false, 0, None)));
        }

        Ok(None)
    }

    /// Build the workspace directory set for one source map.
    fn directory_paths_from_source_map(&self, source: &SourceMap) -> FxHashSet<PathBuf> {
        let mut directories = FxHashSet::default();
        directories.insert(self.root.normalize());

        // physical workspace directories
        for entry in source.values() {
            let Some(path) = self.path_for_origin(&entry.origin) else {
                continue;
            };

            let mut current = path.parent().map(Path::to_path_buf);
            while let Some(directory) = current {
                let directory = directory.normalize();
                if !directory.starts_with(&self.root) {
                    break;
                }

                directories.insert(directory.clone());

                if directory == self.root {
                    break;
                }

                current = directory.parent().map(Path::to_path_buf);
            }
        }

        directories
    }

    /// Build one file view from one revision source binding.
    fn build_file_at_revision(
        &self,
        file_id: FileId,
        origin: &FileOrigin,
        content: Arc<FileContentEntry>,
    ) -> File {
        let path = self.path_for_origin(origin);
        let uri = self.uri_for_origin(origin, path.as_deref());
        let name = self.name_for_origin(origin, path.as_deref());
        let file_type = self.file_type_for_origin(origin, path.as_deref());

        File::from_content(file_id, name, uri, path, file_type, content)
    }

    /// Materialize one physical path for one revision origin when possible.
    pub(crate) fn path_for_origin(&self, origin: &FileOrigin) -> Option<PathBuf> {
        match origin {
            FileOrigin::Workspace { logical_path } => Some(self.root.join(logical_path)),
            FileOrigin::Builtin { .. } | FileOrigin::Synthetic { .. } => None,
        }
    }

    /// Return one module snapshot for one revision and module id.
    pub fn module(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Result<Option<Arc<Module>>, RepositoryError> {
        let workspace = self.workspace(revision)?;
        Ok(workspace.module(module_id).cloned().map(Arc::new))
    }

    /// Return one tsconfig snapshot for one revision and tsconfig id.
    pub fn tsconfig_declaration(
        &self,
        revision: Revision,
        tsconfig_file_id: FileId,
    ) -> Result<Option<Arc<TsConfigDeclaration>>, RepositoryError> {
        let revision_state = self.revision(revision)?;
        let cache = revision_state
            .tsconfig_declarations
            .get_or_init(|| Arc::new(parking_lot::RwLock::new(FxHashMap::default())));

        // cache hit
        if let Some(tsconfig) = cache.read().get(&tsconfig_file_id).cloned() {
            return Ok(tsconfig);
        }

        // parse from source
        let tsconfig = match self.file(revision, tsconfig_file_id)? {
            Some(file) => TsConfigDeclaration::parse(true, &file).ok().map(Arc::new),
            None => None,
        };

        cache.write().insert(tsconfig_file_id, tsconfig.clone());

        Ok(tsconfig)
    }

    /// Access tsconfig for one module via closure.
    pub fn read_tsconfig_declaration_for_module<T>(
        &self,
        revision: Revision,
        module: &Module,
        read: impl FnOnce(&TsConfigDeclaration) -> T,
    ) -> Result<Option<T>, RepositoryError> {
        let Some(tsconfig_file_id) = module.tsconfig_file_id else {
            return Ok(None);
        };
        let Some(tsconfig) = self.tsconfig_declaration(revision, tsconfig_file_id)? else {
            return Ok(None);
        };

        Ok(Some(read(&tsconfig)))
    }

    /// Return tsconfig options for one module.
    pub fn tsconfig_options_for_module(
        &self,
        revision: Revision,
        module: &Module,
    ) -> Result<Option<TsConfigOptions>, RepositoryError> {
        self.read_tsconfig_declaration_for_module(revision, module, |tsconfig| tsconfig.options())
    }

    /// Return one file content identity from one revision.
    pub fn file_content_id(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<FileContentId>, RepositoryError> {
        let revision = self.revision(revision)?;
        Ok(revision.file_content_id(file_id))
    }

    /// Return one file content payload from one revision.
    pub fn file_content(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<Arc<FileContent>>, RepositoryError> {
        let Some(file) = self.file(revision, file_id)? else {
            return Ok(None);
        };

        Ok(Some(Arc::new(file.content.payload().clone())))
    }

    /// Return the tsconfig file id that applies to one module path.
    fn module_tsconfig_file_id_for_path(
        &self,
        revision: Revision,
        path: Option<&Path>,
    ) -> Result<Option<FileId>, RepositoryError> {
        let Some(path) = path else {
            return Ok(None);
        };

        self.applicable_tsconfig_file_id_for_path(revision, path)
    }

    /// Return the effective tsconfig file id for one path.
    pub fn applicable_tsconfig_file_id_for_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<FileId>, RepositoryError> {
        let Some(nearest_tsconfig_file_id) =
            self.nearest_tsconfig_file_id_at_path(revision, path)?
        else {
            return Ok(None);
        };

        Ok(Some(self.select_effective_tsconfig_file_id(
            revision,
            nearest_tsconfig_file_id,
            path,
        )?))
    }

    /// Return the nearest tsconfig file id for one path.
    fn nearest_tsconfig_file_id_at_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<FileId>, RepositoryError> {
        let mut current = path.parent();
        while let Some(directory) = current {
            let tsconfig_path = directory.join("tsconfig.json");
            let tsconfig_file_id = self.file_id_for_workspace_path(&tsconfig_path);
            if self
                .tsconfig_declaration(revision, tsconfig_file_id)?
                .is_some()
            {
                return Ok(Some(tsconfig_file_id));
            }

            current = directory.parent();
        }

        Ok(None)
    }

    /// Return the most specific effective tsconfig file id.
    fn select_effective_tsconfig_file_id(
        &self,
        revision: Revision,
        tsconfig_file_id: FileId,
        path: &Path,
    ) -> Result<FileId, RepositoryError> {
        let mut visited = HashSet::new();

        Ok(self
            .select_effective_tsconfig_file_id_recursive(
                revision,
                tsconfig_file_id,
                path,
                &mut visited,
            )?
            .unwrap_or(tsconfig_file_id))
    }

    /// Walk tsconfig references to find the most specific match.
    fn select_effective_tsconfig_file_id_recursive(
        &self,
        revision: Revision,
        tsconfig_file_id: FileId,
        path: &Path,
        visited: &mut HashSet<FileId>,
    ) -> Result<Option<FileId>, RepositoryError> {
        if !visited.insert(tsconfig_file_id) {
            return Ok(None);
        }

        let Some(tsconfig) = self.tsconfig_declaration(revision, tsconfig_file_id)? else {
            return Ok(None);
        };
        if tsconfig.applies_to_path(path) {
            return Ok(Some(tsconfig_file_id));
        }

        for reference in &tsconfig.json.references {
            let reference_path = tsconfig.directory.normalize_with(&reference.path);
            let Some(reference_tsconfig_file_id) =
                self.tsconfig_file_id_from_reference_path(revision, &reference_path)?
            else {
                continue;
            };

            if let Some(reference_tsconfig_file_id) = self
                .select_effective_tsconfig_file_id_recursive(
                    revision,
                    reference_tsconfig_file_id,
                    path,
                    visited,
                )?
            {
                return Ok(Some(reference_tsconfig_file_id));
            }
        }

        Ok(None)
    }

    /// Resolve one tsconfig reference path to one file id.
    fn tsconfig_file_id_from_reference_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<FileId>, RepositoryError> {
        let Some(path) = self.materialize_tsconfig_path_at(revision, path)? else {
            return Ok(None);
        };

        Ok(Some(self.file_id_for_workspace_path(&path)))
    }

    /// Materialize one tsconfig reference path.
    fn materialize_tsconfig_path_at(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<PathBuf>, RepositoryError> {
        let direct_file_id = self.file_id_for_workspace_path(path);
        if self.file(revision, direct_file_id)?.is_some() {
            return Ok(Some(path.to_path_buf()));
        }

        let nested_path = path.join("tsconfig.json");
        let nested_file_id = self.file_id_for_workspace_path(&nested_path);
        if self.file(revision, nested_file_id)?.is_some() {
            return Ok(Some(nested_path));
        }

        let json_path = PathBuf::from(format!("{}.json", path.display()));
        let json_file_id = self.file_id_for_workspace_path(&json_path);
        if self.file(revision, json_file_id)?.is_some() {
            return Ok(Some(json_path));
        }

        Ok(None)
    }

    /// Return the module ids visible in one revision.
    pub fn workspace_module_ids(
        &self,
        revision: Revision,
    ) -> Result<Vec<ModuleId>, RepositoryError> {
        let workspace = self.workspace(revision)?;
        let mut module_ids = workspace.modules().keys().copied().collect::<Vec<_>>();
        module_ids.sort_unstable();
        module_ids.dedup();
        Ok(module_ids)
    }

    /// Return the module ids visible for one package in one revision.
    pub fn package_module_ids(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Vec<ModuleId>, RepositoryError> {
        let mut module_ids = self
            .workspace_module_ids(revision)?
            .into_iter()
            .filter(|module_id| module_id.package_id == package_id)
            .collect::<Vec<_>>();
        module_ids.sort_unstable();
        module_ids.dedup();
        Ok(module_ids)
    }

    /// Return the module id for one file in one revision when present.
    pub fn module_id_for_file(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<ModuleId>, RepositoryError> {
        let workspace = self.workspace(revision)?;

        for (module_id, module) in workspace.modules().iter() {
            if module.file_id == file_id {
                return Ok(Some(*module_id));
            }
        }

        Ok(None)
    }

    /// Return the module id for one workspace path in one revision when present.
    pub fn module_id_for_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<ModuleId>, RepositoryError> {
        let file_id = self.file_id_for_workspace_path(path);

        self.module_id_for_file(revision, file_id)
    }

    /// Return the module id for one uri in one revision when present.
    pub fn module_id_for_uri(
        &self,
        revision: Revision,
        uri: &Uri,
    ) -> Result<Option<ModuleId>, RepositoryError> {
        let file_id = if let Some(path) = uri.to_path_buf() {
            self.file_id_for_workspace_path(path.as_path())
        } else {
            self.file_id_for_logical_path(uri.as_ref())
        };

        self.module_id_for_file(revision, file_id)
    }

    /// Detect source type for one module file in one revision.
    pub fn detect_module_source_type_at(
        &self,
        revision: Revision,
        path: Option<&Path>,
        package_id: PackageId,
        tsconfig_file_id: Option<FileId>,
        has_import_export: bool,
    ) -> Result<SourceType, RepositoryError> {
        let workspace = self.workspace(revision)?;
        let package = workspace
            .package(package_id)
            .ok_or(RepositoryError::MissingPackage {
                package: package_id,
            })?;

        self.detect_module_source_type_for_package(
            revision,
            path,
            package,
            tsconfig_file_id,
            has_import_export,
        )
    }

    /// Detect source type for one module file in one revision.
    fn detect_module_source_type_for_package(
        &self,
        revision: Revision,
        path: Option<&Path>,
        package: &Package,
        tsconfig_file_id: Option<FileId>,
        has_import_export: bool,
    ) -> Result<SourceType, RepositoryError> {
        let package_type = self.read_package_module_type(revision, package)?;
        let module_detection = self.tsconfig_module_detection_at(revision, tsconfig_file_id)?;

        if let Some(path) = path {
            return Ok(SourceType::detect(
                path,
                has_import_export,
                module_detection,
                package_type.as_deref(),
            ));
        }

        if module_detection == ModuleDetection::Force {
            return Ok(SourceType::Module);
        }

        if let Some(package_type) = package_type.as_deref() {
            if package_type == "module" {
                return Ok(SourceType::Module);
            }

            if package_type == "commonjs" {
                return Ok(SourceType::Script);
            }
        }

        if has_import_export {
            Ok(SourceType::Module)
        } else {
            Ok(SourceType::Script)
        }
    }

    /// Detect module format for one module file in one revision.
    pub fn detect_module_format_at(
        &self,
        revision: Revision,
        path: Option<&Path>,
        language_type: LanguageType,
        source_type: SourceType,
        package_id: PackageId,
        tsconfig_file_id: Option<FileId>,
    ) -> Result<ModuleFormat, RepositoryError> {
        let workspace = self.workspace(revision)?;
        let package = workspace
            .package(package_id)
            .ok_or(RepositoryError::MissingPackage {
                package: package_id,
            })?;

        self.detect_module_format_for_package(
            revision,
            path,
            language_type,
            source_type,
            package,
            tsconfig_file_id,
        )
    }

    /// Detect module format for one module file in one revision.
    fn detect_module_format_for_package(
        &self,
        revision: Revision,
        path: Option<&Path>,
        language_type: LanguageType,
        source_type: SourceType,
        package: &Package,
        tsconfig_file_id: Option<FileId>,
    ) -> Result<ModuleFormat, RepositoryError> {
        let package_type = self.read_package_module_type(revision, package)?;
        let tsconfig_format = self.tsconfig_module_format_at(revision, tsconfig_file_id)?;

        Ok(ModuleFormat::detect(
            path,
            language_type,
            source_type,
            package_type.as_deref(),
            tsconfig_format,
        ))
    }

    /// Drop cached module graphs for one profile.
    pub fn drop_module_graph(&self, profile_id: ProfileId) {
        self.artifacts
            .evict_key(&ArtifactKey::module_graph(profile_id));
    }

    /// Return whether one workspace file should materialize as a module.
    fn is_module_file(&self, path: &Path, file_type: FileType) -> bool {
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };

        if matches!(file_name, "package.json" | "destack.json" | "tsconfig.json") {
            return false;
        }

        file_type.is_code() || file_type.is_data() || file_type.is_text() || file_type.is_binary()
    }

    /// Return one builtin module entry for one file origin when applicable.
    fn builtin_module(&self, file_id: FileId, origin: &FileOrigin) -> Option<Module> {
        let FileOrigin::Builtin { logical_path } = origin else {
            return None;
        };
        let source = self.builtins.module_source_for_path(logical_path)?;

        let file_type = FileType::from_path_or_unknown(Path::new(logical_path));
        let language_type = LanguageType::from(file_type);
        let loader = Loader::from_file_type(file_type);
        let module_id = ModuleId::from_relative_path(BUILTIN_PACKAGE_ID, Path::new(logical_path));

        Some(Module::blank(
            module_id,
            file_id,
            Uri::from_string(format!("builtin://{logical_path}")),
            None,
            BUILTIN_PACKAGE_ID,
            language_type,
            loader,
            source,
        ))
    }

    /// Return one synthetic module entry for one file origin when applicable.
    fn synthetic_module(&self, file_id: FileId, origin: &FileOrigin) -> Option<Module> {
        let FileOrigin::Synthetic { logical_path, .. } = origin else {
            return None;
        };

        if !origin.is_root() {
            return None;
        }

        Some(Module::blank(
            self.synthetic_root_module_id(),
            file_id,
            Uri::from_string(format!("synthetic://{logical_path}")),
            None,
            PackageId::EPHEMERAL,
            LanguageType::Destack,
            Loader::Destack,
            ModuleSource::User,
        ))
    }

    /// Return the uri for one file origin.
    fn uri_for_origin(&self, origin: &FileOrigin, path: Option<&Path>) -> Uri {
        match origin {
            FileOrigin::Workspace { .. } => {
                let path = path.expect("workspace origin should have one path");
                Uri::from_path(path)
            }
            FileOrigin::Builtin { logical_path } => {
                Uri::from_string(format!("builtin://{logical_path}"))
            }
            FileOrigin::Synthetic { logical_path, .. } => {
                Uri::from_string(format!("synthetic://{logical_path}"))
            }
        }
    }

    /// Return the display name for one file origin.
    fn name_for_origin(&self, origin: &FileOrigin, path: Option<&Path>) -> String {
        match origin {
            FileOrigin::Workspace { logical_path } | FileOrigin::Builtin { logical_path } => path
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| logical_path.clone()),
            FileOrigin::Synthetic { logical_path, .. } => logical_path.clone(),
        }
    }

    /// Return the file type for one file origin.
    fn file_type_for_origin(&self, origin: &FileOrigin, path: Option<&Path>) -> FileType {
        match origin {
            FileOrigin::Workspace { logical_path } | FileOrigin::Builtin { logical_path } => path
                .map(FileType::from_path_or_unknown)
                .unwrap_or_else(|| FileType::from_path_or_unknown(Path::new(logical_path))),
            FileOrigin::Synthetic { file_type, .. } => *file_type,
        }
    }
}
