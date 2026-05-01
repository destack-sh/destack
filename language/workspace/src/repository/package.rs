use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{FileId, PackageId, TargetId, Uri};
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
        let package_file_id = package
            .path
            .as_ref()
            .and_then(|path| self.tracked_file_id(files, &path.join("package.json")));
        let destack_file_id = package
            .path
            .as_ref()
            .and_then(|path| self.tracked_file_id(files, &path.join("destack.json")));
        let tsconfig_file_id = package
            .path
            .as_ref()
            .and_then(|path| self.tracked_file_id(files, &path.join("tsconfig.json")));

        let package_declaration = match package_file_id {
            Some(file_id) => self.package_declaration_for_file(revision, file_id)?,
            None => None,
        };
        let destack_declaration = match destack_file_id {
            Some(file_id) => self.destack_declaration_for_file(revision, file_id)?,
            None => None,
        };
        let package_options = destack_declaration
            .as_ref()
            .map(|declaration| declaration.package_options());
        let mut targets = IndexMap::new();

        // explicit targets
        if let Some(package_options) = package_options.as_ref() {
            for (name, options) in &package_options.targets {
                let target_id = TargetId::new(package.id, name);
                let target = options.to_target(name);

                targets.insert(target_id, target);
            }
        }

        let package = Package {
            id: package.id,
            kind: package.kind,
            uri: package.uri.clone(),
            path: package.path.clone(),
            name: package_options
                .as_ref()
                .and_then(|options| options.name.clone())
                .or_else(|| {
                    package_declaration
                        .as_ref()
                        .and_then(|declaration| declaration.name().map(ToOwned::to_owned))
                }),
            version: package_options
                .as_ref()
                .and_then(|options| options.version.clone())
                .or_else(|| {
                    package_declaration
                        .as_ref()
                        .and_then(|declaration| declaration.version().map(ToOwned::to_owned))
                }),
            package_file_id,
            destack_file_id,
            tsconfig_file_id,
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
        let mut physical_roots = HashSet::new();
        let mut base_packages = OrdMap::new();

        // package roots
        for entry in files.values() {
            let path = self.root.join(&entry.logical_path);
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            if matches!(file_name, "package.json" | "destack.json") {
                let package_root = path.parent().unwrap_or(self.root.as_path()).to_path_buf();
                physical_roots.insert(package_root);
            }
        }

        // base packages
        for entry in files.values() {
            let package = self.base_package(&physical_roots, &entry.logical_path);
            base_packages.insert(package.id, package);
        }

        // config enrichment
        let mut packages = OrdMap::new();
        for package in base_packages.values() {
            let package = self.build_package(revision, files, package)?;
            packages.insert(package.id, package);
        }

        Ok(PackageIndex::new(packages))
    }

    /// Return the package index for one revision.
    pub(crate) fn package_index(
        &self,
        revision: Revision,
    ) -> Result<Arc<PackageIndex>, RepositoryError> {
        let revision_state = self.revision(revision)?;
        let revision_cache = self.revision_cache(revision);

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

    /// Return the package root and ownership kind for one workspace file path.
    fn package_scope(
        &self,
        declared_roots: &HashSet<PathBuf>,
        path: &Path,
    ) -> (PackageKind, PathBuf) {
        let directory = path.parent().unwrap_or(self.root.as_path()).to_path_buf();

        for ancestor in directory.ancestors() {
            if !ancestor.starts_with(&self.root) {
                break;
            }

            if declared_roots.contains(ancestor) {
                return (PackageKind::Declared, ancestor.to_path_buf());
            }
        }

        (PackageKind::Implicit, directory)
    }

    /// Return the package id for one package root.
    fn package_id(&self, kind: PackageKind, root: &Path) -> PackageId {
        match kind {
            PackageKind::Declared => PackageId::from_path(root),
            PackageKind::Implicit => PackageId::from_synthetic_path(root),
        }
    }

    /// Return one base package for one logical path.
    fn base_package(&self, declared_roots: &HashSet<PathBuf>, logical_path: &str) -> Package {
        let path = self.root.join(logical_path);
        let (kind, package_root) = self.package_scope(declared_roots, &path);

        Package {
            id: self.package_id(kind, &package_root),
            kind,
            uri: Uri::from_path(&package_root),
            path: Some(package_root),
            name: None,
            version: None,
            package_file_id: None,
            destack_file_id: None,
            tsconfig_file_id: None,
            targets: IndexMap::new(),
        }
    }
}
