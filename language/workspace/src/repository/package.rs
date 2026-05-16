use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{FileId, PackageId, TargetId, Uri, matches as glob_matches};
use im::OrdMap;
use indexmap::IndexMap;

use crate::repository::{FileEntry, Repository, RepositoryError, Revision};
use crate::{Package, PackageIndex, PackageKind};

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
        let mut mode_dependencies = IndexMap::new();

        // explicit targets
        if let Some(config) = config {
            for (name, target) in &config.targets {
                let target_id = TargetId::new(package.id, name);

                targets.insert(target_id, target.clone());
            }

            for (name, mode) in &config.conditions.modes {
                if !mode.dependencies.is_empty() {
                    mode_dependencies.insert(name.clone(), mode.dependencies.clone());
                }
            }
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
            mode_dependencies,
            vendoring: config.map(|config| config.vendoring).unwrap_or_default(),
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

        // config enrichment
        for (package_root, kind) in package_roots {
            let package = self.base_package(kind, &package_root);
            let package = self.build_package(revision, files, &package)?;

            packages.insert(package.id, package);
        }

        Ok(PackageIndex::new(packages))
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
            Self::push_package_root(&mut package_roots, &mut seen, self.root.clone(), kind);

            return Ok(package_roots);
        }

        // explicit workspace packages
        for entry in files.values() {
            let path = self.root.join(&entry.logical_path);
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            if file_name == "destack.json" {
                let package_root = path.parent().unwrap_or(self.root.as_path()).to_path_buf();

                if self.is_workspace_package_root(&package_root, workspace_packages) {
                    Self::push_package_root(
                        &mut package_roots,
                        &mut seen,
                        package_root,
                        PackageKind::Declared,
                    );
                }
            }
        }

        Ok(package_roots)
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
            PackageKind::Declared | PackageKind::Implicit => PackageId::from_path(root),
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
            mode_dependencies: IndexMap::new(),
            vendoring: Default::default(),
            destack_file_id: None,
            targets: IndexMap::new(),
        }
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

        Self::matches_workspace_glob(pattern, relative_root)
            || Self::matches_workspace_glob(pattern, config_path)
    }

    /// Match one workspace glob pattern.
    fn matches_workspace_glob(pattern: &str, path: &str) -> bool {
        glob_matches(pattern.as_bytes(), 0, path.as_bytes(), 0)
    }

    /// Push one package root when it has not been seen before.
    fn push_package_root(
        package_roots: &mut Vec<(PathBuf, PackageKind)>,
        seen: &mut HashSet<PathBuf>,
        package_root: PathBuf,
        kind: PackageKind,
    ) {
        if seen.insert(package_root.clone()) {
            package_roots.push((package_root, kind));
        }
    }
}
