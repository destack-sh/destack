use std::path::Path;
use std::sync::Arc;

use destack_source::{FileId, FileType, LanguageType, Loader, ModuleId, PackageId, Uri};
use im::OrdMap;

use crate::repository::{FileEntry, Repository, RepositoryError, Revision};
use crate::{Module, ModuleDetection, ModuleFormat, ModuleSource, Package, SourceType};

impl Repository {
    /// Build one shared module with package and tsconfig configuration applied.
    fn build_module(
        &self,
        revision: Revision,
        module: &Module,
        packages: &OrdMap<PackageId, Arc<Package>>,
    ) -> Result<Arc<Module>, RepositoryError> {
        let mut module = module.clone();
        let package = packages
            .get(&module.package_id)
            .ok_or(RepositoryError::MissingPackage {
                package: module.package_id,
            })?;
        let tsconfig_file_id = match module.path.as_deref() {
            Some(path) => self.tsconfig_file_id_for_path(revision, path)?,
            None => None,
        };
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

        Ok(Arc::new(module))
    }

    /// Build the module index from one file map.
    pub(crate) fn module_index_for_files(
        &self,
        revision: Revision,
        files: &OrdMap<FileId, FileEntry>,
        packages: &OrdMap<PackageId, Arc<Package>>,
    ) -> Result<OrdMap<ModuleId, Arc<Module>>, RepositoryError> {
        let mut modules = OrdMap::new();
        let package_roots = self.sorted_package_roots(packages);

        for (file_id, entry) in files.iter() {
            let Some(path) = self.path_for_source(&entry.source) else {
                continue;
            };
            let file_type = FileType::from_path_or_unknown(&path);
            if !self.is_module_file(&path, file_type) {
                continue;
            }

            let Some(package) = self.nearest_indexed_package(packages, &package_roots, &path)
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

        let mut module_index = OrdMap::new();

        for module in modules.values() {
            let module = self.build_module(revision, module, packages)?;
            module_index.insert(module.id, module);
        }

        for module in self.builtin_modules() {
            module_index.insert(module.id, Arc::new(module));
        }

        Ok(module_index)
    }

    /// Return one module for one revision and module id.
    pub fn module(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Result<Option<Arc<Module>>, RepositoryError> {
        let workspace = self.workspace(revision)?;
        Ok(workspace.module(module_id))
    }

    /// Return the module ids visible in one revision.
    pub fn module_ids(&self, revision: Revision) -> Result<Vec<ModuleId>, RepositoryError> {
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
            .module_ids(revision)?
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
        let workspace = self.workspace(revision)?;

        for (module_id, module) in workspace.modules().iter() {
            if &module.uri == uri {
                return Ok(Some(*module_id));
            }
        }

        Ok(None)
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
        let package_type = self.package_module_type(revision, package)?;
        let module_detection = self.tsconfig_module_detection(revision, tsconfig_file_id)?;

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
    fn detect_module_format_for_package(
        &self,
        revision: Revision,
        path: Option<&Path>,
        language_type: LanguageType,
        source_type: SourceType,
        package: &Package,
        tsconfig_file_id: Option<FileId>,
    ) -> Result<ModuleFormat, RepositoryError> {
        let package_type = self.package_module_type(revision, package)?;
        let tsconfig_format = self.tsconfig_module_format(revision, tsconfig_file_id)?;

        Ok(ModuleFormat::detect(
            path,
            language_type,
            source_type,
            package_type.as_deref(),
            tsconfig_format,
        ))
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
}
