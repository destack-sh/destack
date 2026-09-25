use std::collections::HashSet;
use std::hash::Hash;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use im::OrdMap;
use indexmap::IndexMap;
use tspp_artifact::{
    ConditionSet, ExportPattern, ExportTarget, PackageDependency, PackageExports, PackageNode,
    PackageSetFingerprint, SourceDependency,
};
use tspp_core::{StableHasher, TreapRoot, stable_hash_value_128};
use tspp_source::{LanguageType, PackageId, TargetId, Uri};

use crate::config::{
    ConditionGate, ConditionRef, DEFAULT_EXPORT_CONDITION, Dependency, Export, MANIFEST_FILE_NAME,
};
use crate::repository::{Repository, RepositoryError, Revision};
use crate::{
    ExportCase, ManifestFile, Package, PackageDependencies, PackageExport, PackageIndex,
    PackageKind,
};

impl Repository {
    /// Return a dependency on one package configuration and its resolved dependency targets.
    pub fn package_dependency(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<SourceDependency, RepositoryError> {
        // distinguish absent, embedded, and authored packages
        let package = self.package(revision, package_id)?;
        let mut hasher = StableHasher::new();
        package.is_some().hash(&mut hasher);
        if let Some(package) = package {
            package.path.hash(&mut hasher);

            // observe inherited configuration files in their declared order
            if let Some(configuration) = &package.configuration {
                for file in &configuration.file_ids {
                    file.hash(&mut hasher);
                    let blob = self.file_blob(revision, *file)?;
                    blob.map(|blob| blob.id).hash(&mut hasher);
                }
            }

            // include unavailable dependencies so later discovery invalidates imports
            let dependencies = package.dependencies.iter().chain(
                package
                    .conditional_dependencies
                    .iter()
                    .flat_map(|group| group.dependencies.iter()),
            );
            for (name, dependency) in dependencies {
                name.hash(&mut hasher);
                dependency.hash(&mut hasher);
                let target = self.dependency_package(revision, &package, name, dependency)?;
                target.map(|package| package.id).hash(&mut hasher);
            }
        }

        Ok(SourceDependency::Package {
            package: package_id,
            fingerprint: hasher.finish_u128(),
        })
    }

    /// Build one Package from an authored package root.
    fn build_package(
        &self,
        revision: Revision,
        discovered_kind: PackageKind,
        package_root: &Path,
    ) -> Result<Arc<Package>, RepositoryError> {
        let config_path = package_root.join(MANIFEST_FILE_NAME);
        let configuration = self
            .inherited_manifest_for_path(revision, &config_path)?
            .map(Arc::new);
        let config = configuration.as_deref();
        let is_builtin = config
            .and_then(|config| config.name.as_deref())
            .is_some_and(|name| name == self.embedded_builtin().package_name());
        let kind = discovered_kind;
        let id = self.package_id(is_builtin, package_root);
        let uri = if is_builtin {
            self.embedded_builtin().package_uri().clone()
        } else {
            Uri::logical(package_root.to_string_lossy())
        };

        // require authored packages to declare their import name
        let name = config
            .and_then(|config| config.name.clone())
            .filter(|name| !name.is_empty());
        if name.is_none() && matches!(kind, PackageKind::Declared | PackageKind::Dependency) {
            return Err(match config {
                None => RepositoryError::MissingPackageConfig { path: config_path },
                Some(_) => RepositoryError::MissingPackageName { path: config_path },
            });
        }
        let mut targets = IndexMap::new();
        let mut conditional_dependencies = Vec::new();
        let mut exports = IndexMap::new();

        // explicit targets
        if let Some(config) = config {
            for (name, target) in &config.targets {
                let target_id = TargetId::new(id, name);

                targets.insert(target_id, target.clone());
            }

            conditional_dependencies.extend(Self::condition_dependencies(config));
            conditional_dependencies.extend(Self::resolve_conditional_dependencies(config)?);
            exports.extend(Self::resolve_exports(config)?);
        }

        let package = Package {
            id,
            kind,
            is_builtin,
            uri,
            path: Some(package_root.to_path_buf()),
            name,
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
            targets,
            configuration,
        };

        Ok(Arc::new(package))
    }

    /// Build the PackageIndex for one file root.
    pub(crate) fn package_index_for_files(
        &self,
        revision: Revision,
        file_root: TreapRoot,
    ) -> Result<PackageIndex, RepositoryError> {
        let workspace = self.manifest_for_workspace(revision)?;
        let package_roots =
            self.package_roots_for_files(revision, file_root, workspace.as_deref())?;
        let mut packages = OrdMap::new();

        // build authored packages from config
        for (package_root, discovered_kind) in package_roots {
            let package = self.build_package(revision, discovered_kind, &package_root)?;
            if package.is_builtin && packages.contains_key(&package.id) {
                return Err(RepositoryError::DuplicatePackageName {
                    name: self.embedded_builtin().package_name().to_string(),
                });
            }

            packages.insert(package.id, package);
        }

        // insert the embedded Builtin Package only when authored sources do not replace it
        if !packages.contains_key(&self.embedded_builtin().package_id()) {
            let package = self.embedded_builtin().package();
            packages.insert(package.id, package);
        }

        Self::reject_duplicate_package_names(packages.values().map(Arc::as_ref))?;

        Ok(PackageIndex::new(packages, workspace.as_deref()))
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

    /// Return package roots for one file bindings.
    fn package_roots_for_files(
        &self,
        revision: Revision,
        file_root: TreapRoot,
        workspace_config: Option<&ManifestFile>,
    ) -> Result<Vec<(PathBuf, PackageKind)>, RepositoryError> {
        let workspace = workspace_config.filter(|config| config.workspaces.is_some());
        let mut package_roots = Vec::new();
        let mut seen = HashSet::new();

        // default workspace package
        if workspace.is_none() {
            let kind = if workspace_config.is_some() {
                PackageKind::Declared
            } else {
                PackageKind::Implicit
            };
            let workspace_root = PathBuf::new();
            if seen.insert(workspace_root.clone()) {
                package_roots.push((workspace_root, kind));
            }
        }
        // explicit workspace packages
        else {
            let files = self.file_entries(revision)?;
            for (file_id, entry) in files.iter().copied() {
                let path = PathBuf::from(self.logical_path_text(entry.logical_path));
                let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                    continue;
                };

                if file_name != MANIFEST_FILE_NAME {
                    continue;
                }

                // retain selected manifests, skipping package.json files of other package managers
                let package_root = path.parent().unwrap_or(Path::new("")).to_path_buf();
                let package_path = package_root.to_string_lossy().replace('\\', "/");
                if workspace.is_some_and(|workspace| workspace.selects_package(&package_path))
                    && self.local_manifest_for_file(revision, file_id)?.is_some()
                    && seen.insert(package_root.clone())
                {
                    package_roots.push((package_root, PackageKind::Declared));
                }
            }
        }

        self.push_module_package_roots(revision, file_root, &mut package_roots, &mut seen)?;

        Ok(package_roots)
    }

    /// Add package roots reachable from captured dependency declarations.
    fn push_module_package_roots(
        &self,
        revision: Revision,
        file_root: TreapRoot,
        package_roots: &mut Vec<(PathBuf, PackageKind)>,
        seen: &mut HashSet<PathBuf>,
    ) -> Result<(), RepositoryError> {
        let mut index = 0;

        while index < package_roots.len() {
            let package_root = package_roots[index].0.clone();
            index += 1;

            let config_path = package_root.join(MANIFEST_FILE_NAME);
            let Some(config) = self.inherited_manifest_for_path(revision, &config_path)? else {
                continue;
            };
            for (package_name, dependency) in Self::declared_dependency_sources(&config) {
                let Some(package_root) =
                    self.dependency_package_root(&package_root, package_name, dependency)
                else {
                    continue;
                };
                if !self.has_manifest(revision, file_root, &package_root)? {
                    continue;
                }

                if seen.insert(package_root.clone()) {
                    package_roots.push((package_root, PackageKind::Dependency));
                }
            }
        }

        Ok(())
    }

    /// Return whether one captured package root has a TS++ manifest.
    fn has_manifest(
        &self,
        revision: Revision,
        file_root: TreapRoot,
        package_root: &Path,
    ) -> Result<bool, RepositoryError> {
        let file_id = self.file_id(&package_root.join(MANIFEST_FILE_NAME));
        if !self.file_tree.contains(file_root, &file_id) {
            return Ok(false);
        }

        Ok(self.local_manifest_for_file(revision, file_id)?.is_some())
    }

    /// Return the package index for one revision.
    pub(crate) fn package_index(
        &self,
        revision: Revision,
    ) -> Result<Arc<PackageIndex>, RepositoryError> {
        let revision_state = self.revision(revision)?;
        let revision_cache = revision_state.cache();

        // return the cached discovery result
        if let Some(packages) = revision_cache.packages.get() {
            return packages.clone();
        }

        // cache the complete package discovery result
        let packages = self
            .package_index_for_files(revision, revision_state.files())
            .map(Arc::new);

        revision_cache.packages.get_or_init(|| packages).clone()
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

    /// Return one package with every package it depends on, transitively, the package first.
    pub fn package_closure(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Vec<PackageId>, RepositoryError> {
        let mut closure = vec![package_id];
        let mut index = 0;
        while index < closure.len() {
            let package =
                self.package(revision, closure[index])?
                    .ok_or(RepositoryError::MissingPackage {
                        package: closure[index],
                    })?;
            let dependencies = package.dependencies.iter().chain(
                package
                    .conditional_dependencies
                    .iter()
                    .flat_map(|group| group.dependencies.iter()),
            );
            for (name, dependency) in dependencies {
                let target = self
                    .dependency_package(revision, &package, name, dependency)?
                    .ok_or_else(|| RepositoryError::UnresolvedDependency {
                        package: package.id,
                        name: name.to_string(),
                    })?;
                if !closure.contains(&target.id) {
                    closure.push(target.id);
                }
            }
            index += 1;
        }

        Ok(closure)
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

    /// Return the package set fingerprint for one revision.
    pub fn packages_fingerprint(
        &self,
        revision: Revision,
    ) -> Result<PackageSetFingerprint, RepositoryError> {
        let state = self.revision(revision)?;
        if let Some(fingerprint) = state.cache().packages_fingerprint.get() {
            return Ok(*fingerprint);
        }
        let fingerprint = PackageSetFingerprint::new(&self.package_ids(revision)?);

        Ok(*state
            .cache()
            .packages_fingerprint
            .get_or_init(|| fingerprint))
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
    fn package_id(&self, is_builtin: bool, root: &Path) -> PackageId {
        if is_builtin {
            self.embedded_builtin().package_id()
        } else {
            PackageId::from_path(root)
        }
    }

    /// Return dependencies declared by named conditions.
    fn condition_dependencies(config: &ManifestFile) -> Vec<PackageDependencies> {
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
        config: &ManifestFile,
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
            Dependency::Registry { version } => {
                Some(Self::mount_root(&format!("{package_name}@{version}")))
            }
            Dependency::Path { path } => {
                Some(self.path_dependency_root(current_root, package_name, path))
            }
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
    /// Dependencies inside the workspace keep workspace logical roots;
    ///  roots escaping the workspace live under their dependency mount.
    fn path_dependency_root(
        &self,
        current_root: &Path,
        package_name: &str,
        path: &Path,
    ) -> PathBuf {
        if path.is_absolute() {
            return Self::mount_root(package_name);
        }

        match normalize_logical_package_path(current_root.join(path)) {
            Some(path) => path,
            None => Self::mount_root(package_name),
        }
    }

    /// Return the logical mount root for one dependency name.
    fn mount_root(name: &str) -> PathBuf {
        PathBuf::from(format!("{}{name}", super::file::MOUNT_PREFIX))
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
        let package_root = Self::mount_root(&format!("git-{source_hash:032x}"));

        match path {
            Some(path) => normalize_package_subpath(&package_root, path),
            None => Some(package_root),
        }
    }

    /// Resolve condition references in conditional dependency declarations.
    fn resolve_conditional_dependencies(
        config: &ManifestFile,
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
        config: &ManifestFile,
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
        config: &ManifestFile,
        export: &Export,
    ) -> Result<PackageExport, RepositoryError> {
        let cases = match export {
            // an unconditional path
            Export::Path(path) => vec![ExportCase {
                when: None,
                path: path.clone(),
            }],
            // one path per condition, the default case unconditional and last
            Export::Conditions(paths) => paths
                .iter()
                .enumerate()
                .map(|(index, (condition, path))| {
                    // reject cases after the default case, which could never apply
                    if condition == DEFAULT_EXPORT_CONDITION && index + 1 != paths.len() {
                        return Err(RepositoryError::InvalidConfig {
                            file: config.file_id,
                            message: format!(
                                "export condition '{DEFAULT_EXPORT_CONDITION}' must come last"
                            ),
                        });
                    }

                    let when = (condition != DEFAULT_EXPORT_CONDITION)
                        .then(|| {
                            config
                                .conditions
                                .resolve(&ConditionRef::Name(condition.clone()))
                        })
                        .transpose()
                        .map_err(|error| RepositoryError::InvalidConfig {
                            file: config.file_id,
                            message: error.to_string(),
                        })?;

                    Ok(ExportCase {
                        when,
                        path: path.clone(),
                    })
                })
                .collect::<Result<_, RepositoryError>>()?,
        };

        Ok(PackageExport { cases })
    }

    /// Build the active import-resolution node of one package.
    pub fn package_node(
        &self,
        revision: Revision,
        package_id: PackageId,
        conditions: &ConditionSet,
    ) -> Result<Option<PackageNode>, RepositoryError> {
        let Some(package) = self.package(revision, package_id)? else {
            return Ok(None);
        };

        // resolve active package dependency declarations
        let mut dependencies = IndexMap::new();
        for (name, dependency) in package.dependencies_for_conditions(conditions) {
            let target = self.dependency_package(revision, &package, &name, &dependency)?;
            let dependency = match target {
                Some(package) => PackageDependency::Resolved(package.id),
                None => PackageDependency::Unavailable,
            };
            dependencies.insert(name, dependency);
        }

        // index the path each export selects under the active conditions
        let mut exact = IndexMap::new();
        let mut patterns = Vec::new();
        for (key, export) in &package.exports {
            let Some(path) = export.select(conditions) else {
                continue;
            };
            let target = ExportTarget {
                path: path.to_string(),
                is_module: LanguageType::from_path(Path::new(path)).is_some(),
            };
            // index wildcard exports separately
            if let Some((prefix, suffix)) = key.split_once('*') {
                patterns.push(ExportPattern {
                    prefix: prefix.to_string(),
                    suffix: suffix.to_string(),
                    target,
                });
            }
            // index exact exports directly
            else {
                exact.insert(key.clone(), target);
            }
        }

        // prefer the most specific pattern before generic catchalls
        patterns.sort_by(|left, right| {
            right
                .prefix
                .len()
                .cmp(&left.prefix.len())
                .then_with(|| right.suffix.len().cmp(&left.suffix.len()))
        });

        Ok(Some(PackageNode {
            root: package.path.clone(),
            dependencies,
            exports: PackageExports { exact, patterns },
        }))
    }
}

/// Normalize one logical package path lexically.
/// Returns none when the path escapes the workspace root.
fn normalize_logical_package_path(path: PathBuf) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            Component::Normal(component) => normalized.push(component),
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }

    Some(normalized)
}

/// Normalize one package store subpath without escaping the package root.
fn normalize_package_subpath(package_root: &Path, path: &Path) -> Option<PathBuf> {
    if path.is_absolute() {
        return None;
    }
    let path = normalize_logical_package_path(package_root.join(path))?;

    path.starts_with(package_root).then_some(path)
}
