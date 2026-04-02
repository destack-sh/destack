use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::Loader;
use destack_source::{
    File, FileContent, FileId, FileType, LanguageType, ModuleId, PackageId, PathExt, ProfileId, Uri,
};
use im::OrdMap;

use crate::repository::{
    BUILTIN_PACKAGE_ID, FileContentId, FileOrigin, Repository, RepositoryError, Revision, SourceMap,
};
use crate::{
    Module, ModuleDetection, ModuleFormat, ModuleSource, Package, SourceType, TsConfigDeclaration,
};

impl Repository {
    /// Return the synthetic root module id.
    pub fn root_module_id(&self) -> ModuleId {
        ModuleId::EPHEMERAL
    }

    /// Derive module views from one source map.
    pub(crate) fn derive_modules(
        &self,
        source: &SourceMap,
        packages: &OrdMap<PackageId, Package>,
    ) -> OrdMap<ModuleId, Module> {
        let mut modules = OrdMap::new();

        for file_id in source.keys() {
            let Some(origin) = self.file_origin_by_file_id(*file_id) else {
                continue;
            };

            // synthetic root module
            if matches!(origin, FileOrigin::Root) {
                let module = Module::blank(
                    self.root_module_id(),
                    *file_id,
                    Uri::from_string("<root>"),
                    None,
                    PackageId::EPHEMERAL,
                    LanguageType::Destack,
                    Loader::Destack,
                    ModuleSource::User,
                );

                modules.insert(module.id, module);
                continue;
            }

            // builtin module
            if let Some(module) = self.builtin_module(*file_id, &origin) {
                modules.insert(module.id, module);
                continue;
            }

            let FileOrigin::Workspace { path } = origin else {
                continue;
            };
            let path = path.to_path_buf();
            let file_type = FileType::from_path_or_unknown(&path);
            if !self.is_module_file(&path, file_type) {
                continue;
            }

            let Some(package) = self.discovered_package_for_path(packages, &path) else {
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

        modules
    }

    /// Return one source file snapshot for one revision and file id.
    pub fn file(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<Arc<File>>, RepositoryError> {
        let revision_id = revision;
        let revision = self.revision(revision_id)?;
        let Some(content_id) = revision.file_content_id(file_id) else {
            return Ok(None);
        };
        let Some(logical_path) = self.logical_path_by_file_id(file_id) else {
            return Ok(None);
        };
        let content = self.file_content_by_id(content_id)?;
        let file = self.build_file_at_revision(file_id, logical_path.as_ref(), content);

        Ok(Some(Arc::new(file)))
    }

    /// Build one file view from one revision source binding.
    fn build_file_at_revision(
        &self,
        file_id: FileId,
        logical_path: &str,
        content: Arc<FileContent>,
    ) -> File {
        let path = self.workspace_path_for_logical_path(logical_path);
        let uri = path
            .as_ref()
            .map(Uri::from_path)
            .unwrap_or_else(|| Uri::from_string(logical_path));
        let name = path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| logical_path.to_string());
        let file_type = if logical_path == "<root>" {
            FileType::Destack
        } else {
            path.as_deref()
                .map(FileType::from_path_or_unknown)
                .unwrap_or_else(|| FileType::from_path_or_unknown(Path::new(logical_path)))
        };
        match content.as_ref() {
            FileContent::Text { content } => {
                if file_type == FileType::Json {
                    File::from_text_as_jsonc(
                        file_id,
                        name.clone(),
                        uri.clone(),
                        path.clone(),
                        file_type,
                        content.clone(),
                    )
                    .unwrap_or_else(|_| {
                        File::from_text(
                            file_id,
                            name.clone(),
                            uri.clone(),
                            path.clone(),
                            file_type,
                            content.clone(),
                        )
                    })
                } else {
                    File::from_text(
                        file_id,
                        name.clone(),
                        uri.clone(),
                        path.clone(),
                        file_type,
                        content.clone(),
                    )
                }
            }
            FileContent::Json { content, .. } => File::from_text_as_jsonc(
                file_id,
                name.clone(),
                uri.clone(),
                path.clone(),
                file_type,
                content.clone(),
            )
            .unwrap_or_else(|error| panic!("invalid stored json content for {file_id:?}: {error}")),
            FileContent::Binary { content } => File::from_binary(
                file_id,
                name.clone(),
                uri.clone(),
                path.clone(),
                file_type,
                content.clone(),
            ),
            FileContent::Missing => {
                File::missing(file_id, name.clone(), uri.clone(), path.clone(), file_type)
            }
            FileContent::Unloaded => {
                File::unloaded(file_id, name.clone(), uri.clone(), path.clone(), file_type)
            }
        }
    }

    /// Materialize one physical path for one revision logical path when possible.
    pub(crate) fn workspace_path_for_logical_path(&self, logical_path: &str) -> Option<PathBuf> {
        if logical_path.contains("://") || logical_path.starts_with('<') {
            return None;
        }

        Some(self.root.join(logical_path))
    }

    /// Return one module snapshot for one revision and module id.
    pub fn module(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Result<Option<Arc<Module>>, RepositoryError> {
        let revision_id = revision;
        let revision = self.revision(revision_id)?;
        let packages = self.derive_packages(revision.source.as_ref());
        let modules = self.derive_modules(revision.source.as_ref(), &packages);
        let mut module = if let Some(module) = modules.get(&module_id) {
            Module::blank(
                module.id,
                module.file_id,
                module.uri.clone(),
                module.path.clone(),
                module.package_id,
                module.language_type,
                module.loader,
                module.source,
            )
        } else {
            return Ok(None);
        };

        if self.file(revision_id, module.file_id)?.is_none() {
            return Ok(None);
        }

        let tsconfig_file_id = self.module_tsconfig_file_id_at(revision_id, &module)?;
        let source_type = self.detect_module_source_type_at(
            revision_id,
            module.path.as_deref(),
            module.package_id,
            tsconfig_file_id,
            false,
        )?;
        let module_format = self.detect_module_format_at(
            revision_id,
            module.path.as_deref(),
            module.language_type,
            source_type,
            module.package_id,
            tsconfig_file_id,
        )?;

        module.tsconfig_file_id = tsconfig_file_id;
        module.source_type = source_type;
        module.module_format = module_format;

        Ok(Some(Arc::new(module)))
    }

    /// Return one tsconfig snapshot for one revision and tsconfig id.
    pub fn tsconfig_declaration(
        &self,
        revision: Revision,
        tsconfig_file_id: FileId,
    ) -> Result<Option<Arc<TsConfigDeclaration>>, RepositoryError> {
        let Some(file) = self.file(revision, tsconfig_file_id)? else {
            return Ok(None);
        };
        let Ok(tsconfig) = TsConfigDeclaration::parse(true, &file) else {
            return Ok(None);
        };

        Ok(Some(Arc::new(tsconfig)))
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
        let Some(content_id) = self.file_content_id(revision, file_id)? else {
            return Ok(None);
        };

        Ok(Some(self.file_content_by_id(content_id)?))
    }

    /// Return the tsconfig file id that applies to one module path.
    fn module_tsconfig_file_id_at(
        &self,
        revision: Revision,
        module: &Module,
    ) -> Result<Option<FileId>, RepositoryError> {
        let Some(path) = module.path.as_ref() else {
            return Ok(None);
        };

        self.applicable_tsconfig_file_id_at_path(revision, path)
    }

    /// Return the effective tsconfig file id for one path.
    fn applicable_tsconfig_file_id_at_path(
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
        let revision = self.revision(revision)?;
        let packages = self.derive_packages(revision.source.as_ref());
        let modules = self.derive_modules(revision.source.as_ref(), &packages);
        let mut module_ids = modules.keys().copied().collect::<Vec<_>>();
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
        let module_ids = self.workspace_module_ids(revision)?;

        for module_id in module_ids {
            let Some(module) = self.module(revision, module_id)? else {
                continue;
            };
            if module.file_id == file_id {
                return Ok(Some(module_id));
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
        let package_type = self.package_module_type_at(revision, package_id)?;
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
        let package_type = self.package_module_type_at(revision, package_id)?;
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
            .evict_key(&destack_artifact::ArtifactKey::module_graph(profile_id));
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

    /// Return the nearest package entry for one path.
    fn discovered_package_for_path<'a>(
        &self,
        packages: &'a OrdMap<PackageId, Package>,
        path: &Path,
    ) -> Option<&'a Package> {
        packages
            .values()
            .filter_map(|package| {
                let package_path = package.path.as_ref()?;
                if !path.starts_with(package_path) {
                    return None;
                }

                Some((package_path.as_os_str().len(), package))
            })
            .max_by_key(|(package_length, _)| *package_length)
            .map(|(_, package)| package)
    }

    /// Return one builtin module entry for one file origin when applicable.
    fn builtin_module(&self, file_id: FileId, origin: &FileOrigin) -> Option<Module> {
        let FileOrigin::Builtin {
            module_path,
            source,
        } = origin
        else {
            return None;
        };

        let file_type = FileType::from_path_or_unknown(Path::new(module_path));
        let language_type = LanguageType::from(file_type);
        let loader = Loader::from_file_type(file_type);
        let module_id = ModuleId::from_relative_path(BUILTIN_PACKAGE_ID, Path::new(module_path));

        Some(Module::blank(
            module_id,
            file_id,
            Uri::from_string(
                self.logical_path_by_file_id(file_id)
                    .unwrap_or_else(|| Arc::<str>::from(format!("builtin://{module_path}")))
                    .as_ref(),
            ),
            None,
            BUILTIN_PACKAGE_ID,
            language_type,
            loader,
            *source,
        ))
    }
}
