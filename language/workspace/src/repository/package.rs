use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::PackageId;
use im::OrdMap;
use indexmap::IndexMap;
use rustc_hash::FxHashMap;

use crate::repository::{
    BUILTIN_PACKAGE_ID, FileOrigin, Repository, RepositoryError, Revision, SourceMap,
};
use crate::{
    DestackDeclaration, Package, PackageDeclaration, PackageKind, PackageOptions, WorkspaceOptions,
};

impl Repository {
    /// Return one parsed package declaration by file id.
    fn package_declaration_by_file_id(
        &self,
        revision: Revision,
        file_id: destack_source::FileId,
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
        file_id: destack_source::FileId,
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

    /// Derive package views from one source map.
    pub(crate) fn derive_packages(&self, source: &SourceMap) -> OrdMap<PackageId, Package> {
        let mut physical_roots = std::collections::HashSet::new();

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

        let mut packages = OrdMap::new();

        // package views
        for (file_id, entry) in source.iter() {
            let Some(mut package) = self.package_for_origin(&physical_roots, &entry.origin) else {
                continue;
            };
            let package_id = package.id;

            if let Some(package_path) = package.path.as_ref()
                && let Some(path) = self.path_for_origin(&entry.origin)
                && path.parent() == Some(package_path.as_path())
                && let Some(file_name) = path.file_name().and_then(|name| name.to_str())
                && file_name == "tsconfig.json"
            {
                package.tsconfig_file_id = Some(*file_id);
            }

            packages.insert(package_id, package);
        }

        packages
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
    pub fn destack_declaration_for_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<Arc<DestackDeclaration>>, RepositoryError> {
        let path = self.materialize_destack_path(revision, path)?;
        let file_id = self.file_id_for_workspace_path(&path);

        self.destack_declaration_by_file_id(revision, file_id)
    }

    /// Find the nearest `destack.json` path for one workspace path.
    pub fn nearest_destack_path(
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
                .destack_declaration_for_path(revision, &candidate)?
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

    /// Return the discovered package paths for one revision.
    pub fn workspace_package_paths(
        &self,
        revision: Revision,
    ) -> Result<Vec<PathBuf>, RepositoryError> {
        let mut package_paths = self
            .workspace_package_ids(revision)?
            .into_iter()
            .filter_map(|package_id| {
                self.package(revision, package_id)
                    .ok()
                    .flatten()
                    .and_then(|package| package.path.clone())
            })
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
        let mut best_package = None;
        let mut best_length = 0usize;

        for package_id in self.workspace_package_ids(revision)? {
            let Some(package) = self.package(revision, package_id)? else {
                continue;
            };
            let Some(package_path) = package.path.as_ref() else {
                continue;
            };
            if !path.starts_with(package_path) {
                continue;
            }

            let package_length = package_path.as_os_str().len();
            if package_length <= best_length {
                continue;
            }

            best_length = package_length;
            best_package = Some(package);
        }

        Ok(best_package)
    }

    /// Return one package snapshot for one revision and package id.
    pub fn package(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Option<Arc<Package>>, RepositoryError> {
        let workspace = self.workspace(revision)?;
        let mut package = if let Some(package) = workspace.package(package_id) {
            package.clone()
        } else {
            return Ok(None);
        };

        let package_declaration = self.package_declaration(revision, &package)?;
        let destack_declaration = self.package_destack_declaration(revision, &package)?;
        let package_options = destack_declaration
            .as_ref()
            .map(|declaration| declaration.package_options());

        package.package_file_id = package_declaration
            .as_ref()
            .map(|declaration| declaration.file_id);
        package.destack_file_id = destack_declaration
            .as_ref()
            .map(|declaration| declaration.file_id);
        package.name = package_options
            .as_ref()
            .and_then(|options| options.name.clone())
            .or_else(|| {
                package_declaration
                    .as_ref()
                    .and_then(|declaration| declaration.name().map(ToOwned::to_owned))
            });
        package.version = package_options
            .as_ref()
            .and_then(|options| options.version.clone())
            .or_else(|| {
                package_declaration
                    .as_ref()
                    .and_then(|declaration| declaration.version().map(ToOwned::to_owned))
            });

        if let Some(package_options) = package_options.as_ref() {
            let mut targets = IndexMap::new();

            // explicit targets
            for (name, options) in &package_options.targets {
                let target_id = self.intern_target_id(package.id, name);
                let target = options.to_target(name);

                targets.insert(target_id, target);
            }

            package.targets = targets;
        }

        Ok(Some(Arc::new(package)))
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
        let Some(package_path) = package.path.as_ref() else {
            return Ok(None);
        };

        let file_id = self.file_id_for_workspace_path(&package_path.join("destack.json"));
        self.destack_declaration_by_file_id(revision, file_id)
    }

    /// Normalize one path into the concrete `destack.json` location it names.
    fn materialize_destack_path(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<PathBuf, RepositoryError> {
        let path = match self.metadata_for_path(revision, path)? {
            Some(metadata) if metadata.is_file => path.to_path_buf(),
            Some(metadata) if metadata.is_directory => path.join("destack.json"),
            _ => {
                let mut os_string = path.to_path_buf().into_os_string();
                os_string.push(".json");
                PathBuf::from(os_string)
            }
        };

        Ok(path)
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

        Ok(self
            .package_destack_declaration(revision, package)?
            .map(|declaration| declaration.package_options()))
    }

    /// Return the package module type for one package.
    pub(crate) fn package_module_type_at(
        &self,
        revision: Revision,
        package_id: PackageId,
    ) -> Result<Option<String>, RepositoryError> {
        // from workspace
        let workspace = self.workspace(revision)?;
        let Some(package) = workspace.package(package_id) else {
            return Ok(None);
        };
        let Some(package_path) = package.path.as_ref() else {
            return Ok(None);
        };

        // destack.json
        let destack_file_id = self.file_id_for_workspace_path(&package_path.join("destack.json"));
        if let Some(declaration) = self.destack_declaration_by_file_id(revision, destack_file_id)?
            && let Some(module_type) = declaration.package_options().module_type
        {
            return Ok(Some(module_type));
        }

        // package.json
        let package_file_id = self.file_id_for_workspace_path(&package_path.join("package.json"));
        if let Some(declaration) = self.package_declaration_by_file_id(revision, package_file_id)? {
            return Ok(declaration.module_type().map(str::to_string));
        }

        Ok(None)
    }

    /// Return the package root and kind for one workspace file path.
    fn package_root_for_path(
        &self,
        physical_roots: &std::collections::HashSet<PathBuf>,
        path: &Path,
    ) -> (PackageKind, PathBuf) {
        let directory = path.parent().unwrap_or(self.root.as_path()).to_path_buf();

        for ancestor in directory.ancestors() {
            if !ancestor.starts_with(&self.root) {
                break;
            }

            if physical_roots.contains(ancestor) {
                return (PackageKind::Physical, ancestor.to_path_buf());
            }
        }

        (PackageKind::Synthetic, directory)
    }

    /// Return the package id for one package root.
    fn package_id_for_root(&self, kind: PackageKind, root: &Path) -> PackageId {
        match kind {
            PackageKind::Physical => PackageId::from_path(root),
            PackageKind::Synthetic => PackageId::from_synthetic_path(root),
            PackageKind::Ephemeral => PackageId::EPHEMERAL,
            PackageKind::Builtin => BUILTIN_PACKAGE_ID,
        }
    }

    /// Return one package entry for one file origin when applicable.
    fn package_for_origin(
        &self,
        physical_roots: &std::collections::HashSet<PathBuf>,
        origin: &FileOrigin,
    ) -> Option<Package> {
        match origin {
            FileOrigin::Synthetic { logical_path, .. } if origin.is_root() => Some(Package {
                id: PackageId::EPHEMERAL,
                kind: PackageKind::Ephemeral,
                uri: destack_source::Uri::from_string(format!("synthetic://{logical_path}")),
                path: None,
                name: None,
                version: None,
                package_file_id: None,
                destack_file_id: None,
                tsconfig_file_id: None,
                targets: Default::default(),
            }),
            FileOrigin::Builtin { .. } => Some(Package {
                id: BUILTIN_PACKAGE_ID,
                kind: PackageKind::Builtin,
                uri: destack_source::Uri::from_string("builtin://"),
                path: None,
                name: None,
                version: None,
                package_file_id: None,
                destack_file_id: None,
                tsconfig_file_id: None,
                targets: Default::default(),
            }),
            FileOrigin::Workspace { .. } => {
                let path = self.path_for_origin(origin)?;
                let (kind, package_root) = self.package_root_for_path(physical_roots, &path);

                Some(Package {
                    id: self.package_id_for_root(kind, &package_root),
                    kind,
                    uri: destack_source::Uri::from_path(&package_root),
                    path: Some(package_root),
                    name: None,
                    version: None,
                    package_file_id: None,
                    destack_file_id: None,
                    tsconfig_file_id: None,
                    targets: Default::default(),
                })
            }
            FileOrigin::Synthetic { .. } => None,
        }
    }
}
