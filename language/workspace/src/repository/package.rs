use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{FileId, PackageId, Uri};
use im::OrdMap;
use indexmap::IndexMap;
use rustc_hash::FxHashMap;

use crate::repository::{
    BUILTIN_PACKAGE_ID, FileOrigin, Repository, RepositoryError, Revision, SourceMap,
};
use crate::{
    DestackDeclaration, Package, PackageDeclaration, PackageKind, PackageOptions, WorkspaceOptions,
};

/// One located package before declaration-driven enrichment.
#[derive(Debug, Clone)]
struct PackageLocator {
    /// The package id.
    id: PackageId,
    /// The package kind.
    kind: PackageKind,
    /// The package uri.
    uri: Uri,
    /// The package path when physical or synthetic.
    path: Option<PathBuf>,
}

impl Repository {
    /// Build package roots ordered from most specific to least specific.
    pub(crate) fn package_paths(
        &self,
        packages: &OrdMap<PackageId, Package>,
    ) -> Vec<(PathBuf, PackageId)> {
        let mut package_paths = packages
            .values()
            .filter_map(|package| package.path.as_ref().map(|path| (path.clone(), package.id)))
            .collect::<Vec<_>>();

        package_paths.sort_by(|left, right| {
            right
                .0
                .as_os_str()
                .len()
                .cmp(&left.0.as_os_str().len())
                .then_with(|| left.0.cmp(&right.0))
        });

        package_paths
    }

    /// Return the nearest package snapshot for one path from one package index.
    pub(crate) fn package_for_indexed_path<'a>(
        &self,
        packages: &'a OrdMap<PackageId, Package>,
        package_paths: &[(PathBuf, PackageId)],
        path: &Path,
    ) -> Option<&'a Package> {
        for (package_path, package_id) in package_paths {
            if path.starts_with(package_path) {
                return packages.get(package_id);
            }
        }

        None
    }

    /// Build one tracked file id for one package-relative file when it exists.
    fn tracked_package_file_id(&self, source: &SourceMap, path: &Path) -> Option<FileId> {
        let file_id = self.file_id_for_workspace_path(path);

        source.contains_key(&file_id).then_some(file_id)
    }

    /// Build one package snapshot from one located package.
    fn package_snapshot_for_locator(
        &self,
        revision: Revision,
        source: &SourceMap,
        locator: &PackageLocator,
    ) -> Result<Package, RepositoryError> {
        let package_file_id = locator
            .path
            .as_ref()
            .and_then(|path| self.tracked_package_file_id(source, &path.join("package.json")));
        let destack_file_id = locator
            .path
            .as_ref()
            .and_then(|path| self.tracked_package_file_id(source, &path.join("destack.json")));
        let tsconfig_file_id = locator
            .path
            .as_ref()
            .and_then(|path| self.tracked_package_file_id(source, &path.join("tsconfig.json")));

        let package_declaration = match package_file_id {
            Some(file_id) => self.package_declaration_by_file_id(revision, file_id)?,
            None => None,
        };
        let destack_declaration = match destack_file_id {
            Some(file_id) => self.destack_declaration_by_file_id(revision, file_id)?,
            None => None,
        };
        let package_options = destack_declaration
            .as_ref()
            .map(|declaration| declaration.package_options());
        let mut targets = IndexMap::new();

        // explicit targets
        if let Some(package_options) = package_options.as_ref() {
            for (name, options) in &package_options.targets {
                let target_id = self.intern_target_id(locator.id, name);
                let target = options.to_target(name);

                targets.insert(target_id, target);
            }
        }

        Ok(Package {
            id: locator.id,
            kind: locator.kind,
            uri: locator.uri.clone(),
            path: locator.path.clone(),
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
        })
    }

    /// Return one parsed package declaration by file id.
    fn package_declaration_by_file_id(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<Arc<PackageDeclaration>>, RepositoryError> {
        let revision_state = self.revision(revision)?;
        let cache = revision_state
            .package_declarations
            .get_or_init(|| Arc::new(parking_lot::RwLock::new(FxHashMap::default())));

        // cache hit
        if let Some(declaration) = cache.read().get(&file_id).cloned() {
            return Ok(declaration);
        }

        // parse from source
        let declaration = match self.file(revision, file_id)? {
            Some(file) => PackageDeclaration::parse(&file).ok().map(Arc::new),
            None => None,
        };

        cache.write().insert(file_id, declaration.clone());

        Ok(declaration)
    }

    /// Return one parsed destack declaration by file id.
    pub(crate) fn destack_declaration_by_file_id(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<Arc<DestackDeclaration>>, RepositoryError> {
        let revision_state = self.revision(revision)?;
        let cache = revision_state
            .destack_declarations
            .get_or_init(|| Arc::new(parking_lot::RwLock::new(FxHashMap::default())));

        // cache hit
        if let Some(declaration) = cache.read().get(&file_id).cloned() {
            return Ok(declaration);
        }

        // parse from source
        let declaration = match self.file(revision, file_id)? {
            Some(file) => DestackDeclaration::parse(&file).ok().map(Arc::new),
            None => None,
        };

        cache.write().insert(file_id, declaration.clone());

        Ok(declaration)
    }

    /// Build package snapshots from one source map.
    pub(crate) fn package_snapshots_from_source_map(
        &self,
        revision: Revision,
        source: &SourceMap,
    ) -> Result<OrdMap<PackageId, Package>, RepositoryError> {
        let mut physical_roots = HashSet::new();
        let mut package_locators = OrdMap::new();

        // package roots
        for entry in source.values() {
            let Some(path) = self.path_for_origin(&entry.origin) else {
                continue;
            };
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            if matches!(file_name, "package.json" | "destack.json") {
                let package_root = path.parent().unwrap_or(self.root.as_path()).to_path_buf();
                physical_roots.insert(package_root);
            }
        }

        // package locators
        for entry in source.values() {
            let Some(locator) = self.package_locator_for_origin(&physical_roots, &entry.origin)
            else {
                continue;
            };

            package_locators.insert(locator.id, locator);
        }

        let mut packages = OrdMap::new();

        // package snapshots
        for locator in package_locators.values() {
            let package = self.package_snapshot_for_locator(revision, source, locator)?;
            packages.insert(package.id, package);
        }

        Ok(packages)
    }

    /// Return the parsed root workspace declaration for one revision.
    pub fn workspace_destack_declaration(
        &self,
        revision: Revision,
    ) -> Result<Option<Arc<DestackDeclaration>>, RepositoryError> {
        let file_id = self.file_id_for_workspace_path(&self.root.join("destack.json"));
        self.destack_declaration_by_file_id(revision, file_id)
    }

    /// Return the parsed `destack.json` declaration for one workspace path.
    pub fn destack_declaration_for_file_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<Arc<DestackDeclaration>>, RepositoryError> {
        let file_id = self.file_id_for_workspace_path(path);

        self.destack_declaration_by_file_id(revision, file_id)
    }

    /// Return the parsed `destack.json` declaration for one directory.
    pub fn destack_declaration_for_directory(
        &self,
        revision: Revision,
        directory: &Path,
    ) -> Result<Option<Arc<DestackDeclaration>>, RepositoryError> {
        let path = directory.join("destack.json");
        self.destack_declaration_for_file_path(revision, &path)
    }

    /// Find the nearest `destack.json` path for one workspace path.
    pub fn nearest_destack_file_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<PathBuf>, RepositoryError> {
        let mut current = match self.metadata_for_path(revision, path)? {
            Some(metadata) if metadata.is_file => path.parent().map(Path::to_path_buf),
            _ => Some(path.to_path_buf()),
        };

        // parent walk
        while let Some(directory) = current {
            let candidate = directory.join("destack.json");

            if self
                .destack_declaration_for_file_path(revision, &candidate)?
                .is_some()
            {
                return Ok(Some(candidate));
            }

            let Some(parent) = directory.parent() else {
                break;
            };
            if parent == directory {
                break;
            }

            current = Some(parent.to_path_buf());
        }

        Ok(None)
    }

    /// Return the effective workspace options for one revision.
    pub fn workspace_options(
        &self,
        revision: Revision,
    ) -> Result<Option<WorkspaceOptions>, RepositoryError> {
        Ok(self
            .workspace_destack_declaration(revision)?
            .map(|declaration| declaration.workspace_options()))
    }

    /// Return the package paths for one revision.
    pub fn workspace_package_paths(
        &self,
        revision: Revision,
    ) -> Result<Vec<PathBuf>, RepositoryError> {
        let workspace = self.workspace(revision)?;
        let mut package_paths = workspace
            .package_paths()
            .iter()
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();

        package_paths.sort();
        package_paths.dedup();

        if package_paths.is_empty() {
            package_paths.push(self.root.clone());
        }

        Ok(package_paths)
    }

    /// Return the nearest package snapshot for one workspace path.
    pub fn package_for_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<Arc<Package>>, RepositoryError> {
        let workspace = self.workspace(revision)?;

        Ok(workspace.package_for_path(path).cloned().map(Arc::new))
    }

    /// Return one package snapshot for one revision and package id.
    pub fn package(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Option<Arc<Package>>, RepositoryError> {
        let workspace = self.workspace(revision)?;
        Ok(workspace.package(package_id).cloned().map(Arc::new))
    }

    /// Return the package ids visible in one revision.
    pub fn workspace_package_ids(
        &self,
        revision: Revision,
    ) -> Result<Vec<PackageId>, RepositoryError> {
        let workspace = self.workspace(revision)?;
        let mut package_ids = workspace.package_ids().collect::<Vec<_>>();
        package_ids.sort_unstable();
        package_ids.dedup();

        Ok(package_ids)
    }

    /// Return the parsed package declaration for one package.
    pub fn package_declaration(
        &self,
        revision: Revision,
        package: &Package,
    ) -> Result<Option<Arc<PackageDeclaration>>, RepositoryError> {
        if let Some(package_file_id) = package.package_file_id {
            return self.package_declaration_by_file_id(revision, package_file_id);
        }

        let Some(package_path) = package.path.as_ref() else {
            return Ok(None);
        };

        let file_id = self.file_id_for_workspace_path(&package_path.join("package.json"));
        self.package_declaration_by_file_id(revision, file_id)
    }

    /// Return the parsed `destack.json` declaration for one package.
    pub fn package_destack_declaration(
        &self,
        revision: Revision,
        package: &Package,
    ) -> Result<Option<Arc<DestackDeclaration>>, RepositoryError> {
        if let Some(destack_file_id) = package.destack_file_id {
            return self.destack_declaration_by_file_id(revision, destack_file_id);
        }

        let Some(package_path) = package.path.as_ref() else {
            return Ok(None);
        };

        let file_id = self.file_id_for_workspace_path(&package_path.join("destack.json"));
        self.destack_declaration_by_file_id(revision, file_id)
    }

    /// Return the effective package options for one package when present.
    pub fn package_options(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Option<PackageOptions>, RepositoryError> {
        let workspace = self.workspace(revision)?;
        let Some(package) = workspace.package(package_id) else {
            return Ok(None);
        };

        self.read_package_options(revision, package)
    }

    /// Return the effective package options for one pinned package snapshot.
    pub(crate) fn read_package_options(
        &self,
        revision: Revision,
        package: &Package,
    ) -> Result<Option<PackageOptions>, RepositoryError> {
        Ok(self
            .package_destack_declaration(revision, package)?
            .map(|declaration| declaration.package_options()))
    }

    /// Return the package module type for one pinned package snapshot.
    pub(crate) fn read_package_module_type(
        &self,
        revision: Revision,
        package: &Package,
    ) -> Result<Option<String>, RepositoryError> {
        if let Some(declaration) = self.package_destack_declaration(revision, package)?
            && let Some(module_type) = declaration.package_options().module_type
        {
            return Ok(Some(module_type));
        }

        if let Some(declaration) = self.package_declaration(revision, package)? {
            return Ok(declaration.module_type().map(str::to_string));
        }

        Ok(None)
    }

    /// Return the package root and ownership kind for one workspace file path.
    fn package_scope_for_path(
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
    fn package_id_for_root(&self, kind: PackageKind, root: &Path) -> PackageId {
        match kind {
            PackageKind::Declared => PackageId::from_path(root),
            PackageKind::Implicit => PackageId::from_synthetic_path(root),
            PackageKind::Ephemeral => PackageId::EPHEMERAL,
            PackageKind::Builtin => BUILTIN_PACKAGE_ID,
        }
    }

    /// Return one located package for one file origin when applicable.
    fn package_locator_for_origin(
        &self,
        declared_roots: &HashSet<PathBuf>,
        origin: &FileOrigin,
    ) -> Option<PackageLocator> {
        match origin {
            FileOrigin::Synthetic { logical_path, .. } if origin.is_root() => {
                Some(PackageLocator {
                    id: PackageId::EPHEMERAL,
                    kind: PackageKind::Ephemeral,
                    uri: Uri::from_string(format!("synthetic://{logical_path}")),
                    path: None,
                })
            }
            FileOrigin::Builtin { .. } => Some(PackageLocator {
                id: BUILTIN_PACKAGE_ID,
                kind: PackageKind::Builtin,
                uri: Uri::from_string("builtin://"),
                path: None,
            }),
            FileOrigin::Workspace { .. } => {
                let path = self.path_for_origin(origin)?;
                let (kind, package_root) = self.package_scope_for_path(declared_roots, &path);

                Some(PackageLocator {
                    id: self.package_id_for_root(kind, &package_root),
                    kind,
                    uri: Uri::from_path(&package_root),
                    path: Some(package_root),
                })
            }
            FileOrigin::Synthetic { .. } => None,
        }
    }
}
