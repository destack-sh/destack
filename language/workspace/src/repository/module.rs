use std::collections::hash_map::Entry;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{FileId, FileType, LanguageType, Loader, ModuleId, PackageId, Uri};
use im::OrdMap;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::repository::{FileEntry, Repository, RepositoryError, Revision};
use crate::{Module, ModuleFile, ModuleIndex, PackageIndex, builtin_mode_names};

/// One file before it is assigned to its canonical module.
#[derive(Debug)]
struct ModuleFileCandidate {
    /// The source file id.
    file_id: FileId,
    /// The source file path.
    path: PathBuf,
    /// The source file type.
    file_type: FileType,
    /// The mode suffix when this is a mode file.
    mode: Option<String>,
    /// The owning package id.
    package_id: PackageId,
    /// The owning package root.
    package_root: Option<PathBuf>,
}

impl Repository {
    /// Build the module index from one file map.
    pub(crate) fn module_index_for_files(
        &self,
        revision: Revision,
        files: &OrdMap<FileId, FileEntry>,
        packages: &PackageIndex,
    ) -> Result<ModuleIndex, RepositoryError> {
        let mut base_files = FxHashMap::default();
        let mut mode_files: FxHashMap<PathBuf, Vec<ModuleFileCandidate>> = FxHashMap::default();
        let mut known_modes = FxHashMap::default();

        // collect base files and their mode files
        for (file_id, entry) in files.iter() {
            let path = self.root.join(&entry.logical_path);
            let Some(candidate) =
                self.module_file_candidate(revision, *file_id, path, packages, &mut known_modes)?
            else {
                continue;
            };

            if let Some(mode) = candidate.mode.as_ref() {
                if let Some(base_path) =
                    Self::mode_base_path(&candidate.path, candidate.file_type, mode)
                {
                    mode_files.entry(base_path).or_default().push(candidate);
                }
            } else {
                base_files.insert(candidate.path.clone(), candidate);
            }
        }

        // build modules from base files only
        let mut module_index = OrdMap::new();
        for (base_path, base) in base_files {
            let mut module = self.module_from_candidate(base);
            if let Some(mut files) = mode_files.remove(&base_path) {
                files.sort_by(|left, right| {
                    left.mode
                        .cmp(&right.mode)
                        .then_with(|| left.path.cmp(&right.path))
                });

                for file in files {
                    module.push_mode_file(Self::module_file_from_candidate(file));
                }
            }

            module_index.insert(module.id, Arc::new(module));
        }

        Ok(ModuleIndex::new(module_index))
    }

    /// Build one candidate from one revision file entry when applicable.
    fn module_file_candidate(
        &self,
        revision: Revision,
        file_id: FileId,
        path: PathBuf,
        packages: &PackageIndex,
        known_modes: &mut FxHashMap<PackageId, FxHashSet<String>>,
    ) -> Result<Option<ModuleFileCandidate>, RepositoryError> {
        let file_type = FileType::from_path_or_unknown(&path);

        // skip non-module workspace files
        if !self.is_module_file(&path, file_type) {
            return Ok(None);
        }

        let Some(package) = packages.nearest_package(&path) else {
            return Ok(None);
        };
        let known_modes = match known_modes.entry(package.id) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let modes = self.known_modes_for_package(revision, package.id)?;

                entry.insert(modes)
            }
        };
        let mode = Self::mode_for_path(&path, file_type, known_modes);

        Ok(Some(ModuleFileCandidate {
            file_id,
            path,
            file_type,
            mode,
            package_id: package.id,
            package_root: package.path.clone(),
        }))
    }

    /// Build one module from its base file candidate.
    fn module_from_candidate(&self, candidate: ModuleFileCandidate) -> Module {
        let language_type = LanguageType::try_from(candidate.file_type).ok();
        let loader = Loader::from(candidate.file_type);
        let module_id = ModuleId::from_path_with_loader(
            candidate.package_id,
            &candidate.path,
            candidate.package_root.as_deref(),
            None,
        );

        Module::blank(
            module_id,
            candidate.file_id,
            Uri::from_path(&candidate.path),
            Some(candidate.path),
            candidate.package_id,
            language_type,
            loader,
        )
    }

    /// Build one module file from a mode file candidate.
    fn module_file_from_candidate(candidate: ModuleFileCandidate) -> ModuleFile {
        let language_type = LanguageType::try_from(candidate.file_type).ok();
        let loader = Loader::from(candidate.file_type);

        ModuleFile::new(
            candidate.file_id,
            Uri::from_path(&candidate.path),
            Some(candidate.path),
            language_type,
            loader,
            candidate.mode,
        )
    }

    /// Return the known mode names for one package.
    fn known_modes_for_package(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<FxHashSet<String>, RepositoryError> {
        let modes =
            if let Some(config) = self.destack_config_for_package_id(revision, package_id)? {
                config.modes.keys().cloned().collect()
            } else {
                builtin_mode_names()
                    .iter()
                    .map(|mode_name| (*mode_name).to_string())
                    .collect()
            };

        Ok(modes)
    }

    /// Return the mode suffix for one path when it has one.
    fn mode_for_path(
        path: &Path,
        file_type: FileType,
        known_modes: &FxHashSet<String>,
    ) -> Option<String> {
        let extension = file_type.extension()?;
        let file_name = path.file_name()?.to_str()?;
        let suffix = format!(".{extension}");
        let stem = file_name.strip_suffix(&suffix)?;
        let (_, mode) = stem.rsplit_once('.')?;

        known_modes.contains(mode).then(|| mode.to_string())
    }

    /// Return the base path for one mode file.
    fn mode_base_path(path: &Path, file_type: FileType, mode: &str) -> Option<PathBuf> {
        let extension = file_type.extension()?;
        let path_text = path.as_os_str().to_string_lossy();
        let mode_suffix = format!(".{mode}.{extension}");
        let base = path_text.strip_suffix(&mode_suffix)?;

        Some(PathBuf::from(format!("{base}.{extension}")))
    }

    /// Return the module index for one revision.
    pub(crate) fn module_index(
        &self,
        revision: Revision,
    ) -> Result<Arc<ModuleIndex>, RepositoryError> {
        let revision_state = self.revision(revision)?;
        let revision_cache = revision_state.cache();

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
        let modules = self.module_index(revision)?;

        Ok(modules.package_module_ids(package_id).to_vec())
    }

    /// Return the module id for one file in one revision when present.
    pub fn module_id_for_file(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<ModuleId>, RepositoryError> {
        let modules = self.module_index(revision)?;

        Ok(modules.module_id_for_file(file_id))
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

        Ok(modules.module_id_for_uri(uri))
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
