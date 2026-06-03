use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_core::stable_hash_value_128;
use destack_source::{FileId, PackageId, TargetId, Uri, matches as glob_matches};
use im::OrdMap;
use indexmap::IndexMap;

use crate::config::{ConditionGate, Dependency, Export};
use crate::repository::{FileEntry, Repository, RepositoryError, Revision};
use crate::{DestackFile, Package, PackageDependencies, PackageExport, PackageIndex, PackageKind};

const DEFAULT_REGISTRY_DIRECTORY: &str = "default";
const GIT_PACKAGE_DIRECTORY: &str = "git";
const REGISTRY_PACKAGE_DIRECTORY: &str = "registry";

impl Repository {
    /// Build one tracked file id for one package-relative file when it exists.
    fn tracked_file_id(&self, files: &OrdMap<FileId, FileEntry>, path: &Path) -> Option<FileId> {
        let file_id = self.file_id(path);

        files.contains_key(&file_id).then_some(file_id)
    }

    /// Build one shared package with declaration configuration applied.
    fn build_package(
        &self,
        revision: Revision,
        files: &OrdMap<FileId, FileEntry>,
        package: &Package,
    ) -> Result<Arc<Package>, RepositoryError> {
        let destack_file_id = package
            .path
            .as_ref()
            .and_then(|path| self.tracked_file_id(files, &path.join("destack.json")));
        let destack_config = package
            .path
            .as_ref()
            .map(|path| self.inherited_destack_for_path(revision, &path.join("destack.json")))
            .transpose()?
            .flatten()
            .map(Arc::new);
        let config = destack_config.as_deref();
        let mut targets = IndexMap::new();
        let mut conditional_dependencies = Vec::new();
        let mut exports = IndexMap::new();

        // explicit targets
        if let Some(config) = config {
            for (name, target) in &config.targets {
                let target_id = TargetId::new(package.id, name);

                targets.insert(target_id, target.clone());
            }

            conditional_dependencies.extend(Self::condition_dependencies(config));
            conditional_dependencies.extend(Self::resolve_conditional_dependencies(config)?);
            exports.extend(Self::resolve_exports(config)?);
        }

        let package = Package {
            id: package.id,
            kind: package.kind,
            uri: package.uri.clone(),
            path: package.path.clone(),
            name: config.and_then(|config| config.name.clone()),
            version: config.and_then(|config| config.version.clone()),
            dependencies: config
                .map(|config| config.dependencies.clone())
                .unwrap_or_default(),
            conditional_dependencies,
            vendor: config
                .map(|config| config.vendor.clone())
                .unwrap_or_default(),
            exports,
            topology: config
                .map(|config| config.topology.clone())
                .unwrap_or_default(),
            destack_file_id,
            targets,
        };

        Ok(Arc::new(package))
    }

    /// Build the package index from one file map.
    pub(crate) fn package_index_for_files(
        &self,
        revision: Revision,
        files: &OrdMap<FileId, FileEntry>,
    ) -> Result<PackageIndex, RepositoryError> {
        let package_roots = self.package_roots_for_files(revision, files)?;
        let mut packages = OrdMap::new();

        // include the immutable builtin package
        let package = self.builtin.package();
        packages.insert(package.id, package);

        // enrich editable workspace packages from config
        for (package_root, kind) in package_roots {
            let package = self.base_package(kind, &package_root);
            let package = self.build_package(revision, files, &package)?;

            packages.insert(package.id, package);
        }

        Self::reject_duplicate_package_names(packages.values().map(Arc::as_ref))?;

        Ok(PackageIndex::new(packages))
    }

    /// Reject duplicate package names used by package specifier imports.
    fn reject_duplicate_package_names<'a>(
        packages: impl IntoIterator<Item = &'a Package>,
    ) -> Result<(), RepositoryError> {
        let mut names = HashSet::new();

        for package in packages {
            let Some(name) = package.name.as_deref() else {
                continue;
            };

            if !names.insert(name.to_string()) {
                return Err(RepositoryError::DuplicatePackageName {
                    name: name.to_string(),
                });
            }
        }

        Ok(())
    }

    /// Return package roots for one file map.
    fn package_roots_for_files(
        &self,
        revision: Revision,
        files: &OrdMap<FileId, FileEntry>,
    ) -> Result<Vec<(PathBuf, PackageKind)>, RepositoryError> {
        let workspace_config = self.destack_for_workspace(revision)?;
        let workspace_packages = workspace_config
            .as_ref()
            .and_then(|config| config.workspace_packages());
        let mut package_roots = Vec::new();
        let mut seen = HashSet::new();

        // default workspace package
        if workspace_packages.is_none() {
            let kind = if workspace_config.is_some() {
                PackageKind::Declared
            } else {
                PackageKind::Implicit
            };
            if seen.insert(self.root.clone()) {
                package_roots.push((self.root.clone(), kind));
            }
        }
        // explicit workspace packages
        else {
            for entry in files.values() {
                let path = self.file_entry_path(entry);
                let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };

                if file_name == "destack.json" {
                    let package_root = path.parent().unwrap_or(self.root.as_path()).to_path_buf();
                    if self.is_workspace_package_root(&package_root, workspace_packages)
                        && seen.insert(package_root.clone())
                    {
                        package_roots.push((package_root, PackageKind::Declared));
                    }
                }
            }
        }

        self.push_dependency_package_roots(revision, files, &mut package_roots, &mut seen)?;

        Ok(package_roots)
    }

    /// Add package roots reachable from captured dependency declarations.
    fn push_dependency_package_roots(
        &self,
        revision: Revision,
        files: &OrdMap<FileId, FileEntry>,
        package_roots: &mut Vec<(PathBuf, PackageKind)>,
        seen: &mut HashSet<PathBuf>,
    ) -> Result<(), RepositoryError> {
        let mut index = 0;

        while index < package_roots.len() {
            let package_root = package_roots[index].0.clone();
            index += 1;

            let config_path = package_root.join("destack.json");
            let Some(config) = self.inherited_destack_for_path(revision, &config_path)? else {
                continue;
            };
            for (package_name, dependency) in Self::declared_dependency_sources(&config) {
                let Some(package_root) =
                    self.dependency_package_root(&package_root, package_name, dependency)
                else {
                    continue;
                };
                if !self.has_package_config(files, &package_root) {
                    continue;
                }

                if seen.insert(package_root.clone()) {
                    package_roots.push((package_root, PackageKind::Dependency));
                }
            }
        }

        Ok(())
    }

    /// Return true when one captured package root has a manifest.
    fn has_package_config(&self, files: &OrdMap<FileId, FileEntry>, package_root: &Path) -> bool {
        let file_id = self.file_id(&package_root.join("destack.json"));

        files.contains_key(&file_id)
    }

    /// Return the package index for one revision.
    pub(crate) fn package_index(
        &self,
        revision: Revision,
    ) -> Result<Arc<PackageIndex>, RepositoryError> {
        let revision_state = self.revision(revision)?;
        let revision_cache = revision_state.cache();

        if let Some(packages) = revision_cache.packages.get() {
            return Ok(Arc::clone(packages));
        }

        let packages =
            Arc::new(self.package_index_for_files(revision, revision_state.files.as_ref())?);
        let packages = revision_cache.packages.get_or_init(|| packages);

        Ok(Arc::clone(packages))
    }

    /// Return the nearest package for one workspace path.
    pub fn nearest_package(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<Arc<Package>>, RepositoryError> {
        let packages = self.package_index(revision)?;

        Ok(packages.nearest_package(path))
    }

    /// Return one package for one revision and package id.
    pub fn package(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Option<Arc<Package>>, RepositoryError> {
        let packages = self.package_index(revision)?;

        Ok(packages.package(package_id))
    }

    /// Return one package by declared package name.
    pub fn package_by_name(
        &self,
        revision: Revision,
        name: &str,
    ) -> Result<Option<Arc<Package>>, RepositoryError> {
        let packages = self.package_index(revision)?;

        Ok(packages.package_by_name(name))
    }

    /// Return one package by package root path.
    pub fn package_by_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<Arc<Package>>, RepositoryError> {
        let packages = self.package_index(revision)?;

        Ok(packages.package_by_path(path))
    }

    /// Return one dependency package by declaration source.
    pub fn dependency_package(
        &self,
        revision: Revision,
        current_package: &Package,
        package_name: &str,
        dependency: &Dependency,
    ) -> Result<Option<Arc<Package>>, RepositoryError> {
        match dependency {
            // workspace dependencies are named package edges
            Dependency::Workspace => self.package_by_name(revision, package_name),

            // source dependencies are rooted by their materialized source path
            Dependency::Registry { .. } | Dependency::Path { .. } | Dependency::Git { .. } => {
                let Some(current_root) = current_package.path.as_deref() else {
                    return Err(RepositoryError::MissingPackagePath {
                        package: current_package.id,
                    });
                };
                let Some(package_root) =
                    self.dependency_package_root(current_root, package_name, dependency)
                else {
                    return Ok(None);
                };

                self.package_by_path(revision, &package_root)
            }
        }
    }

    /// Return one display string for one package id.
    pub fn package_display(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Option<String>, RepositoryError> {
        let Some(package) = self.package(revision, package_id)? else {
            return Ok(None);
        };
        let name = package
            .name
            .clone()
            .or_else(|| package.path.as_ref().map(|path| path.display().to_string()));

        Ok(name)
    }

    /// Return the package ids visible in one revision.
    pub fn package_ids(&self, revision: Revision) -> Result<Vec<PackageId>, RepositoryError> {
        let packages = self.package_index(revision)?;
        let mut package_ids = packages.package_ids().collect::<Vec<_>>();
        package_ids.sort_unstable();
        package_ids.dedup();

        Ok(package_ids)
    }

    /// Return the package id for one package root.
    fn package_id(&self, kind: PackageKind, root: &Path) -> PackageId {
        match kind {
            PackageKind::Builtin => self.builtin.package_id(),
            PackageKind::Declared | PackageKind::Dependency | PackageKind::Implicit => {
                PackageId::from_path(root)
            }
        }
    }

    /// Return one base package for one package root.
    fn base_package(&self, kind: PackageKind, package_root: &Path) -> Package {
        Package {
            id: self.package_id(kind, package_root),
            kind,
            uri: Uri::from_path(package_root),
            path: Some(package_root.to_path_buf()),
            name: None,
            version: None,
            dependencies: IndexMap::new(),
            conditional_dependencies: Vec::new(),
            vendor: Default::default(),
            exports: IndexMap::new(),
            topology: Default::default(),
            destack_file_id: None,
            targets: IndexMap::new(),
        }
    }

    /// Return dependencies declared by named conditions.
    fn condition_dependencies(config: &DestackFile) -> Vec<PackageDependencies> {
        let mut dependencies = Vec::new();

        for (name, condition) in &config.conditions.modes {
            if !condition.dependencies.is_empty() {
                dependencies.push(PackageDependencies {
                    when: ConditionGate::mode(name.clone()),
                    dependencies: condition.dependencies.clone(),
                });
            }
        }
        for (name, condition) in &config.conditions.roles {
            if !condition.dependencies.is_empty() {
                dependencies.push(PackageDependencies {
                    when: ConditionGate::role(name.clone()),
                    dependencies: condition.dependencies.clone(),
                });
            }
        }
        for (name, condition) in &config.conditions.features {
            if !condition.dependencies.is_empty() {
                dependencies.push(PackageDependencies {
                    when: ConditionGate::feature(name.clone()),
                    dependencies: condition.dependencies.clone(),
                });
            }
        }
        for (name, condition) in &config.conditions.tags {
            if !condition.dependencies.is_empty() {
                dependencies.push(PackageDependencies {
                    when: ConditionGate::tag(name.clone()),
                    dependencies: condition.dependencies.clone(),
                });
            }
        }

        dependencies
    }

    /// Return all dependency sources that may become package roots.
    fn declared_dependency_sources(
        config: &DestackFile,
    ) -> impl Iterator<Item = (&String, &Dependency)> {
        let dependencies = config.dependencies.iter();
        let condition_dependencies = config
            .conditions
            .modes
            .values()
            .chain(config.conditions.roles.values())
            .chain(config.conditions.features.values())
            .chain(config.conditions.tags.values())
            .flat_map(|condition| condition.dependencies.iter());
        let conditional_dependencies = config
            .conditional_dependencies
            .iter()
            .flat_map(|conditional| conditional.dependencies.iter());
        let overrides = config.overrides.iter();

        dependencies
            .chain(condition_dependencies)
            .chain(conditional_dependencies)
            .chain(overrides)
    }

    /// Return the captured package root implied by one dependency.
    fn dependency_package_root(
        &self,
        current_root: &Path,
        package_name: &str,
        dependency: &Dependency,
    ) -> Option<PathBuf> {
        match dependency {
            Dependency::Workspace => None,
            Dependency::Registry { registry, version } => {
                Some(self.registry_package_root(package_name, registry.as_deref(), version))
            }
            Dependency::Path { path } => Some(self.path_dependency_root(current_root, path)),
            Dependency::Git {
                url,
                path,
                rev,
                tag,
                branch,
            } => self.git_package_root(url, path.as_deref(), rev, tag, branch),
        }
    }

    /// Return one local path dependency root.
    fn path_dependency_root(&self, current_root: &Path, path: &Path) -> PathBuf {
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            current_root.join(path)
        };

        normalize_package_path(path)
    }

    /// Return one materialized git dependency root.
    fn git_package_root(
        &self,
        url: &str,
        path: Option<&Path>,
        rev: &Option<String>,
        tag: &Option<String>,
        branch: &Option<String>,
    ) -> Option<PathBuf> {
        let source = (url, rev.as_deref(), tag.as_deref(), branch.as_deref());
        let source_hash = stable_hash_value_128(&source);
        let package_root = self
            .layout
            .packages
            .join(GIT_PACKAGE_DIRECTORY)
            .join(format!("{source_hash:032x}"));

        match path {
            Some(path) => normalize_package_subpath(&package_root, path),
            None => Some(package_root),
        }
    }

    /// Return one materialized registry dependency root.
    fn registry_package_root(
        &self,
        package_name: &str,
        registry: Option<&str>,
        version: &str,
    ) -> PathBuf {
        let registry = registry
            .or(self.settings.registry.as_deref())
            .unwrap_or(DEFAULT_REGISTRY_DIRECTORY);

        self.layout
            .packages
            .join(REGISTRY_PACKAGE_DIRECTORY)
            .join(registry)
            .join(package_name)
            .join(version)
    }

    /// Resolve condition references in conditional dependency declarations.
    fn resolve_conditional_dependencies(
        config: &DestackFile,
    ) -> Result<Vec<PackageDependencies>, RepositoryError> {
        let mut dependencies = Vec::new();

        for conditional in &config.conditional_dependencies {
            let when = config
                .conditions
                .resolve(&conditional.when)
                .map_err(|error| RepositoryError::InvalidConfig {
                    file: config.file_id,
                    message: error.to_string(),
                })?;

            dependencies.push(PackageDependencies {
                when,
                dependencies: conditional.dependencies.clone(),
            });
        }

        Ok(dependencies)
    }

    /// Resolve condition references in package export declarations.
    fn resolve_exports(
        config: &DestackFile,
    ) -> Result<IndexMap<String, PackageExport>, RepositoryError> {
        let mut exports = IndexMap::new();

        for (specifier, export) in &config.exports {
            let export = Self::resolve_export(config, export)?;

            exports.insert(specifier.clone(), export);
        }

        Ok(exports)
    }

    /// Resolve one package export declaration.
    fn resolve_export(
        config: &DestackFile,
        export: &Export,
    ) -> Result<PackageExport, RepositoryError> {
        let when = export
            .when
            .as_ref()
            .map(|reference| config.conditions.resolve(reference))
            .transpose()
            .map_err(|error| RepositoryError::InvalidConfig {
                file: config.file_id,
                message: error.to_string(),
            })?;

        Ok(PackageExport {
            kind: export.kind,
            path: export.path.clone(),
            when,
        })
    }

    /// Return true when one package root is selected by workspace config.
    fn is_workspace_package_root(
        &self,
        package_root: &Path,
        workspace_packages: Option<&[String]>,
    ) -> bool {
        match workspace_packages {
            Some(patterns) => patterns
                .iter()
                .any(|pattern| self.matches_workspace_package_pattern(package_root, pattern)),
            None => package_root == self.root,
        }
    }

    /// Return true when one workspace package pattern matches one root.
    fn matches_workspace_package_pattern(&self, package_root: &Path, pattern: &str) -> bool {
        let relative_root = package_root
            .strip_prefix(&self.root)
            .unwrap_or(package_root)
            .to_string_lossy()
            .replace('\\', "/");
        let relative_root = if relative_root.is_empty() {
            "."
        } else {
            relative_root.as_str()
        };
        let config_path = format!("{relative_root}/destack.json");
        let config_path = if relative_root == "." {
            "destack.json"
        } else {
            config_path.as_str()
        };

        glob_matches(pattern.as_bytes(), 0, relative_root.as_bytes(), 0)
            || glob_matches(pattern.as_bytes(), 0, config_path.as_bytes(), 0)
    }
}

/// Normalize one package path lexically.
fn normalize_package_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::Normal(component) => normalized.push(component),
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }

    normalized
}

/// Normalize one package store subpath without escaping the package root.
fn normalize_package_subpath(package_root: &Path, path: &Path) -> Option<PathBuf> {
    if path.is_absolute() {
        return None;
    }
    let path = normalize_package_path(package_root.join(path));

    path.starts_with(package_root).then_some(path)
}
