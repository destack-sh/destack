use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use im::OrdMap;
use indexmap::IndexMap;
use rustc_hash::FxHashMap;
use tspp_artifact::{ModuleSetFingerprint, SourceDependency};
use tspp_source::{
    FileId, FileType, LanguageType, Loader, ModuleId, PackageId, TSPP_FILE_TYPES, Uri,
};

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
    /// The source loader.
    loader: Loader,
    /// The condition alias suffixes for this file.
    aliases: Vec<ConditionFileAlias>,
    /// The owning package id.
    package_id: PackageId,
    /// The owning package root.
    package_root: Option<PathBuf>,
}

/// Decision of one module path against the repository module set.
#[derive(Debug)]
pub struct ModulePathResolution {
    /// The candidate paths inspected.
    pub candidates: usize,
    /// The probes recorded against each candidate path.
    pub probes: Vec<SourceDependency>,
    /// The resolution outcome.
    pub outcome: ModulePathOutcome,
}

/// Outcome of one module path resolution.
#[derive(Debug)]
pub enum ModulePathOutcome {
    /// The path escapes the logical workspace root.
    Unsupported,
    /// No candidate path names a module.
    Missing,
    /// Exactly one candidate path names a module.
    Resolved {
        /// The exact path that selected the module.
        path: PathBuf,
        /// The selected module.
        module: ModuleId,
    },
    /// Multiple candidate paths name modules.
    Ambiguous {
        /// The matching paths and modules.
        matches: Vec<(PathBuf, ModuleId)>,
    },
}

impl Repository {
    /// Return a dependency on one module's contributing files and loader configuration.
    pub fn module_dependency(
        &self,
        revision: Revision,
        module: ModuleId,
    ) -> Result<SourceDependency, RepositoryError> {
        let modules = self.module_index(revision)?;

        Ok(modules.dependency(module))
    }

    /// Build modules from editable revision files.
    pub(crate) fn build_modules(
        &self,
        revision: Revision,
        files: &[(FileId, FileEntry)],
        packages: &PackageIndex,
    ) -> Result<OrdMap<ModuleId, Arc<Module>>, RepositoryError> {
        let mut base_files = FxHashMap::default();
        let mut condition_files: FxHashMap<PathBuf, Vec<ModuleFileCandidate>> =
            FxHashMap::default();
        let mut known_aliases = FxHashMap::default();

        // collect base files and their conditional files
        for (file_id, entry) in files.iter().copied() {
            let path = PathBuf::from(self.logical_path_text(entry.logical_path));
            let Some(candidate) =
                self.module_file_candidate(revision, file_id, path, packages, &mut known_aliases)?
            else {
                continue;
            };

            if candidate.aliases.is_empty() {
                base_files.insert(candidate.path.clone(), candidate);
            } else if let Some(base_path) =
                Self::condition_base_path(&candidate.path, candidate.file_type, &candidate.aliases)
            {
                condition_files
                    .entry(base_path)
                    .or_default()
                    .push(candidate);
            }
        }

        // build modules from base files only
        let mut modules = OrdMap::<ModuleId, Arc<Module>>::new();
        for (base_path, base) in base_files {
            let mut module = self.module_from_candidate(base)?;
            if let Some(mut files) = condition_files.remove(&base_path) {
                files.sort_by(|left, right| {
                    left.aliases
                        .len()
                        .cmp(&right.aliases.len())
                        .then_with(|| left.aliases.cmp(&right.aliases))
                        .then_with(|| left.path.cmp(&right.path))
                });

                for file in files {
                    module.push_condition_file(self.module_file_from_candidate(file)?);
                }
            }

            // narrowed content ids must never collide across distinct modules
            if let Some(previous) = modules.get(&module.id)
                && previous.uri != module.uri
            {
                return Err(RepositoryError::ModuleIdCollision {
                    id: module.id,
                    left: previous.uri.to_string(),
                    right: module.uri.to_string(),
                });
            }

            modules.insert(module.id, Arc::new(module));
        }

        Ok(modules)
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
        let file_type = FileType::from_path(&path).ok_or_else(|| RepositoryError::InvalidFile {
            file: file_id,
            message: format!("module path must name a file: {}", path.display()),
        })?;
        let Ok(loader) = Loader::try_from(file_type) else {
            return Ok(None);
        };

        // skip the package configuration file
        let is_package_config = path.file_name().is_some_and(|name| name == "destack.json");
        if is_package_config {
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
            loader,
            aliases,
            package_id: package.id,
            package_root: package.path.clone(),
        }))
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
                .suffix_aliases()
                .map_err(|error| RepositoryError::InvalidConfig {
                    file: config.file_id,
                    message: error.to_string(),
                })?
        } else {
            builtin_condition_aliases()
        };

        Ok(aliases)
    }

    /// Build one module from its base file candidate.
    fn module_from_candidate(
        &self,
        candidate: ModuleFileCandidate,
    ) -> Result<Module, RepositoryError> {
        let language_type = LanguageType::try_from(candidate.file_type).ok();
        let (module_id, uri) = match candidate.package_root.as_deref() {
            Some(package_root)
                if self.is_builtin_package(candidate.package_id) && language_type.is_some() =>
            {
                let module_id = self.embedded_builtin().module_id_for_path(
                    &candidate.path,
                    package_root,
                    None,
                )?;
                let uri = self
                    .embedded_builtin()
                    .module_uri_for_path(&candidate.path, package_root)?;

                (module_id, uri)
            }
            _ => {
                let module_id = ModuleId::from_path_with_loader(
                    candidate.package_id,
                    &candidate.path,
                    candidate.package_root.as_deref(),
                    None,
                );
                let uri = Uri::logical(candidate.path.to_string_lossy());

                (module_id, uri)
            }
        };

        Ok(Module::blank(
            module_id,
            candidate.file_id,
            uri,
            Some(candidate.path),
            candidate.package_id,
            language_type,
            candidate.loader,
        ))
    }

    /// Build one module file from a conditional file candidate.
    fn module_file_from_candidate(
        &self,
        candidate: ModuleFileCandidate,
    ) -> Result<ModuleFile, RepositoryError> {
        let language_type = LanguageType::try_from(candidate.file_type).ok();
        let uri = match candidate.package_root.as_deref() {
            Some(package_root)
                if self.is_builtin_package(candidate.package_id) && language_type.is_some() =>
            {
                self.embedded_builtin()
                    .module_uri_for_path(&candidate.path, package_root)?
            }
            _ => Uri::logical(candidate.path.to_string_lossy()),
        };
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

        Ok(ModuleFile::new(
            candidate.file_id,
            uri,
            Some(candidate.path),
            language_type,
            candidate.loader,
            aliases,
            gates,
        ))
    }

    /// Return condition aliases for one path when it has any.
    fn condition_aliases_for_path(
        path: &Path,
        file_type: FileType,
        known_aliases: &IndexMap<String, ConditionGate>,
    ) -> Vec<ConditionFileAlias> {
        // skip files that cannot compose source modules
        if LanguageType::try_from(file_type).is_err() {
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

        // walk known aliases while preserving one base name segment
        let mut aliases = Vec::new();
        let mut segments = stem.rsplit('.').peekable();
        while let Some(segment) = segments.next() {
            if segments.peek().is_none() {
                break;
            }
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

        // return the cached discovery result
        if let Some(modules) = revision_cache.modules.get() {
            return modules.clone();
        }

        // cache the complete module discovery result
        let modules = self.package_index(revision).and_then(|packages| {
            let files = self.file_entries(revision)?;

            self.module_index_for_files(revision, files.as_ref(), packages.as_ref())
                .map(Arc::new)
        });

        revision_cache.modules.get_or_init(|| modules).clone()
    }

    /// Build the module index over one file listing and package index.
    pub(crate) fn module_index_for_files(
        &self,
        revision: Revision,
        files: &[(FileId, FileEntry)],
        packages: &PackageIndex,
    ) -> Result<ModuleIndex, RepositoryError> {
        let mut modules = self.build_modules(revision, files, packages)?;

        // include embedded modules only when no authored Builtin Package replaces them
        let package = packages
            .package(self.embedded_builtin().package_id())
            .ok_or(RepositoryError::MissingPackage {
                package: self.embedded_builtin().package_id(),
            })?;
        if package.path.is_none() {
            modules.extend(self.embedded_builtin().modules());
        }

        Ok(ModuleIndex::new(modules))
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

    /// Return the module set fingerprint for one revision.
    pub fn modules_fingerprint(
        &self,
        revision: Revision,
    ) -> Result<ModuleSetFingerprint, RepositoryError> {
        let state = self.revision(revision)?;
        if let Some(fingerprint) = state.cache().modules_fingerprint.get() {
            return Ok(*fingerprint);
        }
        let fingerprint = ModuleSetFingerprint::new(&self.module_ids(revision)?);

        Ok(*state
            .cache()
            .modules_fingerprint
            .get_or_init(|| fingerprint))
    }

    /// Return the module ids visible in one revision.
    pub fn module_ids(&self, revision: Revision) -> Result<Vec<ModuleId>, RepositoryError> {
        let modules = self.module_index(revision)?;
        let mut module_ids = modules.module_ids().collect::<Vec<_>>();
        module_ids.sort_unstable();
        module_ids.dedup();

        Ok(module_ids)
    }

    /// Return the module set fingerprint of one package in one revision.
    pub fn package_modules_fingerprint(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<ModuleSetFingerprint, RepositoryError> {
        let modules = self.module_index(revision)?;

        Ok(ModuleSetFingerprint::new(
            modules.package_module_ids(package_id),
        ))
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

    /// Return the module ID for one URI in one revision when present.
    pub fn module_id_for_uri(
        &self,
        revision: Revision,
        uri: &Uri,
    ) -> Result<Option<ModuleId>, RepositoryError> {
        let modules = self.module_index(revision)?;

        Ok(modules.module_id_for_uri(uri))
    }

    /// Resolve one canonical source URI to its module and file IDs.
    pub fn resolve_uri(
        &self,
        revision: Revision,
        uri: &Uri,
    ) -> Result<Option<(ModuleId, FileId)>, RepositoryError> {
        let modules = self.module_index(revision)?;

        Ok(modules.resolve_uri(uri))
    }

    /// Resolve one module path, recording every candidate probe.
    pub fn resolve_module_path(
        &self,
        revision: Revision,
        path: &Path,
        loader: Option<Loader>,
    ) -> Result<ModulePathResolution, RepositoryError> {
        let Some(path) = normalize_workspace_path(path.to_path_buf()) else {
            return Ok(ModulePathResolution {
                candidates: 0,
                probes: Vec::new(),
                outcome: ModulePathOutcome::Unsupported,
            });
        };
        let paths = module_candidate_paths(path, loader);
        let candidates = paths.len();
        let mut probes = Vec::with_capacity(candidates);
        let mut matches = Vec::new();

        // resolve and record every candidate through the repository module set
        for path in paths {
            let module = self.module_id_for_path(revision, &path)?;
            probes.push(SourceDependency::module_path(self.file_id(&path), module));
            if let Some(module) = module {
                matches.push((path, module));
            }
        }

        let outcome = match matches.len() {
            0 => ModulePathOutcome::Missing,
            1 => {
                let (path, module) = matches.remove(0);

                ModulePathOutcome::Resolved { path, module }
            }
            _ => ModulePathOutcome::Ambiguous { matches },
        };

        Ok(ModulePathResolution {
            candidates,
            probes,
            outcome,
        })
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
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ConditionFileAlias {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank
            .cmp(&other.rank)
            .then_with(|| self.name.cmp(&other.name))
    }
}

/// Normalize one workspace logical path.
pub fn normalize_workspace_path(path: PathBuf) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();

    // fold lexical path components
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            Component::Normal(component) => normalized.push(component),
            Component::RootDir | Component::Prefix(_) => {}
        }
    }

    Some(normalized)
}

/// Return candidate module paths in deterministic order.
fn module_candidate_paths(path: PathBuf, loader: Option<Loader>) -> Vec<PathBuf> {
    let is_recognized =
        FileType::from_path(&path).is_some_and(|file_type| file_type != FileType::Binary);

    // retain recognized source and loader paths
    if is_recognized || loader.is_some() && path.extension().is_some() {
        vec![path]
    }
    // apply an explicit loader extension
    else if let Some(extension) = loader.and_then(Loader::extension) {
        vec![path.with_extension(extension)]
    }
    // try each source extension
    else {
        TSPP_FILE_TYPES
            .iter()
            .filter_map(|file_type| file_type.extension())
            .map(|extension| append_extension(&path, extension))
            .collect()
    }
}

/// Append one extension without replacing a dotted basename suffix.
fn append_extension(path: &Path, extension: &str) -> PathBuf {
    let mut path = path.as_os_str().to_os_string();
    path.push(".");
    path.push(extension);

    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::{ConditionSelector, builtin_condition_aliases};

    use super::*;

    #[test]
    fn test_find_condition_aliases_from_chained_suffixes() {
        let mut aliases = IndexMap::new();
        aliases.insert("test".to_string(), ConditionGate::mode("test"));
        aliases.insert(
            "browser".to_string(),
            ConditionGate {
                host: Some(ConditionSelector::exact("browser")),
                ..ConditionGate::default()
            },
        );

        // preserve suffix order and declaration rank
        let file_aliases = Repository::condition_aliases_for_path(
            Path::new("src/user.test.browser.tspp"),
            FileType::Tspp,
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
    fn test_find_builtin_host_and_runtime_aliases() {
        let aliases = builtin_condition_aliases();

        // recognize target environment suffixes without package aliases
        let file_aliases = Repository::condition_aliases_for_path(
            Path::new("src/user.browser.js.tspp"),
            FileType::Tspp,
            &aliases,
        );
        let names = file_aliases
            .iter()
            .map(|alias| alias.name.as_str())
            .collect::<Vec<_>>();
        let gates = file_aliases
            .iter()
            .map(|alias| alias.gate.clone())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["browser", "js"]);
        assert_eq!(
            gates,
            vec![
                ConditionGate {
                    host: Some(ConditionSelector::exact("browser")),
                    ..ConditionGate::default()
                },
                ConditionGate {
                    runtime: Some(ConditionSelector::exact("js")),
                    ..ConditionGate::default()
                }
            ]
        );
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
            Path::new("src/user.test.d.tspp"),
            FileType::TsppDeclaration,
            &aliases,
        );

        assert_eq!(base_path, Some(PathBuf::from("src/user.d.tspp")));
    }

    #[test]
    fn test_skip_unknown_condition_suffixes() {
        let aliases = IndexMap::new();
        let file_aliases = Repository::condition_aliases_for_path(
            Path::new("src/user.preview.tspp"),
            FileType::Tspp,
            &aliases,
        );

        assert!(file_aliases.is_empty());
    }
}
