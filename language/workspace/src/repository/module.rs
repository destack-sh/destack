use std::collections::hash_map::Entry;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{FileId, FileType, LanguageType, Loader, ModuleId, PackageId, Uri};
use im::OrdMap;
use indexmap::IndexMap;
use rustc_hash::FxHashMap;

use crate::repository::{FileEntry, Repository, RepositoryError, Revision};
use crate::{
    ConditionGate, Module, ModuleFile, ModuleIndex, PackageIndex, builtin_condition_aliases,
};

/// One file before it is assigned to its canonical module.
#[derive(Debug)]
struct ModuleFileCandidate {
    /// The source file id.
    file_id: FileId,
    /// The source file path.
    path: PathBuf,
    /// The source file type.
    file_type: FileType,
    /// The condition alias suffixes for this file.
    aliases: Vec<ConditionFileAlias>,
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
        let mut condition_files: FxHashMap<PathBuf, Vec<ModuleFileCandidate>> =
            FxHashMap::default();
        let mut known_aliases = FxHashMap::default();

        // collect base files and their conditional files
        for (file_id, entry) in files.iter() {
            let path = self.root.join(&entry.logical_path);
            let Some(candidate) =
                self.module_file_candidate(revision, *file_id, path, packages, &mut known_aliases)?
            else {
                continue;
            };

            if candidate.aliases.is_empty() {
                base_files.insert(candidate.path.clone(), candidate);
            } else {
                let Some(base_path) = Self::condition_base_path(
                    &candidate.path,
                    candidate.file_type,
                    &candidate.aliases,
                ) else {
                    continue;
                };

                condition_files
                    .entry(base_path)
                    .or_default()
                    .push(candidate);
            }
        }

        // build modules from base files only
        let mut module_index = OrdMap::new();
        for (base_path, base) in base_files {
            let mut module = self.module_from_candidate(base);
            if let Some(mut files) = condition_files.remove(&base_path) {
                files.sort_by(|left, right| {
                    left.aliases
                        .len()
                        .cmp(&right.aliases.len())
                        .then_with(|| left.aliases.cmp(&right.aliases))
                        .then_with(|| left.path.cmp(&right.path))
                });

                for file in files {
                    module.push_condition_file(Self::module_file_from_candidate(file));
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
        known_aliases: &mut FxHashMap<PackageId, IndexMap<String, ConditionGate>>,
    ) -> Result<Option<ModuleFileCandidate>, RepositoryError> {
        let file_type = FileType::from_path_or_unknown(&path);

        // skip non-module workspace files
        if !self.is_module_file(&path, file_type) {
            return Ok(None);
        }

        let Some(package) = packages.nearest_package(&path) else {
            return Ok(None);
        };
        let known_aliases = match known_aliases.entry(package.id) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let aliases = self.condition_aliases_for_package(revision, package.id)?;

                entry.insert(aliases)
            }
        };
        let aliases = Self::condition_aliases_for_path(&path, file_type, known_aliases);

        Ok(Some(ModuleFileCandidate {
            file_id,
            path,
            file_type,
            aliases,
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

    /// Build one module file from a conditional file candidate.
    fn module_file_from_candidate(candidate: ModuleFileCandidate) -> ModuleFile {
        let language_type = LanguageType::try_from(candidate.file_type).ok();
        let loader = Loader::from(candidate.file_type);
        let aliases = candidate
            .aliases
            .iter()
            .map(|alias| alias.name.clone())
            .collect();
        let gates = candidate
            .aliases
            .into_iter()
            .map(|alias| alias.gate)
            .collect();

        ModuleFile::new(
            candidate.file_id,
            Uri::from_path(&candidate.path),
            Some(candidate.path),
            language_type,
            loader,
            aliases,
            gates,
        )
    }

    /// Return the known condition aliases for one package.
    fn condition_aliases_for_package(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<IndexMap<String, ConditionGate>, RepositoryError> {
        let aliases = if let Some(config) = self.destack_for_package_id(revision, package_id)? {
            config
                .conditions
                .aliases
                .iter()
                .map(|(name, alias)| (name.clone(), alias.clone()))
                .collect()
        } else {
            builtin_condition_aliases()
        };

        Ok(aliases)
    }

    /// Return condition aliases for one path when it has any.
    fn condition_aliases_for_path(
        path: &Path,
        file_type: FileType,
        known_aliases: &IndexMap<String, ConditionGate>,
    ) -> Vec<ConditionFileAlias> {
        // skip files that cannot compose source modules
        if !file_type.is_code() {
            return Vec::new();
        }

        // strip the loader extension before reading suffixes
        let Some(extension) = file_type.extension() else {
            return Vec::new();
        };
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            return Vec::new();
        };
        let suffix = format!(".{extension}");
        let Some(stem) = file_name.strip_suffix(&suffix) else {
            return Vec::new();
        };

        // walk known aliases from the right edge
        let mut aliases = Vec::new();
        for segment in stem.rsplit('.') {
            let Some((rank, name, alias)) = known_aliases.get_full(segment) else {
                break;
            };

            aliases.push(ConditionFileAlias {
                rank,
                name: name.clone(),
                gate: alias.clone(),
            });
        }
        aliases.reverse();

        aliases
    }

    /// Return the base path for one conditional file.
    fn condition_base_path(
        path: &Path,
        file_type: FileType,
        aliases: &[ConditionFileAlias],
    ) -> Option<PathBuf> {
        let extension = file_type.extension()?;
        let path_text = path.as_os_str().to_string_lossy();
        let alias_suffix = aliases
            .iter()
            .map(|alias| alias.name.as_str())
            .collect::<Vec<_>>()
            .join(".");
        let suffix = format!(".{alias_suffix}.{extension}");
        let base = path_text.strip_suffix(&suffix)?;

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

/// One condition alias used by a physical module file.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConditionFileAlias {
    /// The declaration rank of this alias.
    rank: usize,
    /// The alias suffix name.
    name: String,
    /// The condition gate matched by this alias.
    gate: ConditionGate,
}

impl PartialOrd for ConditionFileAlias {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ConditionFileAlias {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.rank
            .cmp(&other.rank)
            .then_with(|| self.name.cmp(&other.name))
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    #[test]
    fn test_find_condition_aliases_from_chained_suffixes() {
        let mut aliases = IndexMap::new();
        aliases.insert("test".to_string(), ConditionGate::mode("test"));
        aliases.insert(
            "browser".to_string(),
            ConditionGate {
                host: Some(crate::ConditionSelector::exact("browser")),
                ..ConditionGate::default()
            },
        );

        // preserve suffix order and declaration rank
        let file_aliases = Repository::condition_aliases_for_path(
            Path::new("src/user.test.browser.ds"),
            FileType::Destack,
            &aliases,
        );
        let names = file_aliases
            .iter()
            .map(|alias| alias.name.as_str())
            .collect::<Vec<_>>();
        let ranks = file_aliases
            .iter()
            .map(|alias| alias.rank)
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["test", "browser"]);
        assert_eq!(ranks, vec![0, 1]);
    }

    #[test]
    fn test_find_condition_base_path_for_compound_extensions() {
        let aliases = vec![ConditionFileAlias {
            rank: 0,
            name: "test".to_string(),
            gate: ConditionGate::mode("test"),
        }];

        // strip aliases before preserving the compound declaration extension
        let base_path = Repository::condition_base_path(
            Path::new("src/user.test.d.ds"),
            FileType::Destack,
            &aliases,
        );

        assert_eq!(base_path, Some(PathBuf::from("src/user.d.ds")));
    }

    #[test]
    fn test_skip_unknown_condition_suffixes() {
        let aliases = IndexMap::new();
        let file_aliases = Repository::condition_aliases_for_path(
            Path::new("src/user.preview.ds"),
            FileType::Destack,
            &aliases,
        );

        assert!(file_aliases.is_empty());
    }
}
