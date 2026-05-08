use destack_source::{FileId, FileType, LanguageType, Loader, ModuleId, PackageId, Uri};
use im::OrdMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::repository::{FileEntry, Repository, RepositoryError, Revision};
use crate::{Module, ModuleIndex, PackageIndex};

impl Repository {
    /// Build the module index from one file map.
    pub(crate) fn module_index_for_files(
        &self,
        _revision: Revision,
        files: &OrdMap<FileId, FileEntry>,
        packages: &PackageIndex,
    ) -> Result<ModuleIndex, RepositoryError> {
        let mut module_index = OrdMap::new();

        for (file_id, entry) in files.iter() {
            let path = self.root.join(&entry.logical_path);
            let Some(module) = self.module_from_file_entry(*file_id, path, packages) else {
                continue;
            };

            module_index.insert(module.id, Arc::new(module));
        }

        Ok(ModuleIndex::new(module_index))
    }

    /// Build one module from one revision file entry when applicable.
    fn module_from_file_entry(
        &self,
        file_id: FileId,
        path: PathBuf,
        packages: &PackageIndex,
    ) -> Option<Module> {
        let file_type = FileType::from_path_or_unknown(&path);

        // skip non-module workspace files
        if !self.is_module_file(&path, file_type) {
            return None;
        }

        let package = packages.nearest_package(&path)?;
        let language_type = LanguageType::try_from(file_type).ok();
        let loader = Loader::from(file_type);
        let module_id =
            ModuleId::from_path_with_loader(package.id, &path, package.path.as_deref(), None);

        Some(Module::blank(
            module_id,
            file_id,
            Uri::from_path(&path),
            Some(path),
            package.id,
            language_type,
            loader,
        ))
    }

    /// Return the module index for one revision.
    pub(crate) fn module_index(
        &self,
        revision: Revision,
    ) -> Result<Arc<ModuleIndex>, RepositoryError> {
        let revision_state = self.revision(revision)?;
        let revision_cache = self.revision_cache(revision);

        if let Some(modules) = revision_cache.modules.get() {
            return Ok(Arc::clone(modules));
        }

        let packages = self.package_index(revision)?;
        let modules = Arc::new(self.module_index_for_files(
            revision,
            revision_state.files.as_ref(),
            packages.as_ref(),
        )?);
        let modules = revision_cache.modules.get_or_init(|| modules);

        Ok(Arc::clone(modules))
    }

    /// Return one module for one revision and module id.
    pub fn module(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Result<Option<Arc<Module>>, RepositoryError> {
        let modules = self.module_index(revision)?;

        Ok(modules.module(module_id))
    }

    /// Return one display string for one module id.
    pub fn module_display(
        &self,
        revision: Revision,
        module_id: ModuleId,
    ) -> Result<Option<String>, RepositoryError> {
        let Some(module) = self.module(revision, module_id)? else {
            return Ok(None);
        };

        Ok(Some(module.uri.to_string()))
    }

    /// Return the module ids visible in one revision.
    pub fn module_ids(&self, revision: Revision) -> Result<Vec<ModuleId>, RepositoryError> {
        let modules = self.module_index(revision)?;
        let mut module_ids = modules.module_ids().collect::<Vec<_>>();
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
        let modules = self.module_index(revision)?;

        for (module_id, module) in modules.iter() {
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
        let file_id = self.file_id(path);

        self.module_id_for_file(revision, file_id)
    }

    /// Return the module id for one uri in one revision when present.
    pub fn module_id_for_uri(
        &self,
        revision: Revision,
        uri: &Uri,
    ) -> Result<Option<ModuleId>, RepositoryError> {
        let modules = self.module_index(revision)?;

        for (module_id, module) in modules.iter() {
            if &module.uri == uri {
                return Ok(Some(*module_id));
            }
        }

        Ok(None)
    }

    /// Return whether one workspace file should materialize as a module.
    fn is_module_file(&self, path: &Path, file_type: FileType) -> bool {
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };

        if file_name == "destack.json" {
            return false;
        }

        file_type.is_code() || file_type.is_data() || file_type.is_text() || file_type.is_binary()
    }
}
