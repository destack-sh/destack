use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::Loader;
use destack_source::{FileContent, FileId, FileType, LanguageType, ModuleId, PackageId, Uri};
use im::OrdMap;
use smallvec::smallvec;

use crate::repository::{BUILTIN_PACKAGE_ID, FileOrigin, Repository, RepositoryError};
use crate::revision::{Change, Edit, Ref, Revision, RevisionData, SourceMap};
use crate::{Module, ModuleSource, Package, PackageKind};

impl Repository {
    /// Seed the synthetic root source into one ref.
    pub fn seed_root_source(&self, reference: &Ref) -> Result<Revision, RepositoryError> {
        let mut seeded_source = SourceMap::new();

        self.insert_seeded_source(
            &mut seeded_source,
            "<root>",
            FileContent::Text {
                content: String::new(),
            },
        );

        self.apply_seeded_source(reference, seeded_source)
    }

    /// Import one filesystem rooted source state into one ref.
    pub fn import_from_fs(
        &self,
        reference: &Ref,
        root: &Path,
    ) -> Result<Revision, RepositoryError> {
        let base_revision_id = self.current(reference)?;
        let base_revision = self.revision(base_revision_id)?;
        let mut source = base_revision.source.as_ref().clone();
        let imported_source = self.collect_source_from_fs(root)?;

        for (file_id, content_id) in imported_source {
            source.insert(file_id, content_id);
        }

        let revision = Arc::new(self.build_revision_data(smallvec![base_revision_id], source));
        let revision_id = revision.revision();

        self.revisions.insert(revision_id, revision);
        self.refs.insert(reference.clone(), revision_id);

        Ok(revision_id)
    }

    /// Create one new ref pointing at another ref's current revision.
    pub fn fork(&self, from: &Ref, to: Ref) -> Result<Revision, RepositoryError> {
        let revision = self.current(from)?;
        self.refs.insert(to, revision);

        Ok(revision)
    }

    /// Publish one new revision that reuses the current source snapshot.
    pub fn publish(&self, reference: &Ref) -> Result<Revision, RepositoryError> {
        self.apply(reference, Change::empty())
    }

    /// Apply one change set to one ref and publish one new revision.
    pub fn apply<C>(&self, reference: &Ref, change: C) -> Result<Revision, RepositoryError>
    where
        C: Into<Change>,
    {
        let base_revision_id = self.current(reference)?;
        let revision_id = self.apply_to_revision(base_revision_id, change)?;
        self.refs.insert(reference.clone(), revision_id);

        Ok(revision_id)
    }

    /// Apply one change set to one base revision and publish one anonymous revision.
    pub fn apply_to_revision<C>(
        &self,
        base_revision_id: Revision,
        change: C,
    ) -> Result<Revision, RepositoryError>
    where
        C: Into<Change>,
    {
        let base_revision = self.revision(base_revision_id)?;
        let source = base_revision.source.as_ref().clone();
        let source = self.apply_change(source, change.into())?;
        let revision = Arc::new(self.build_revision_data(smallvec![base_revision_id], source));
        let revision_id = revision.revision();

        self.revisions
            .entry(revision_id)
            .or_insert_with(|| Arc::clone(&revision));

        Ok(revision_id)
    }

    /// Build one source snapshot from the attached file system.
    fn collect_source_from_fs(&self, root: &Path) -> Result<SourceMap, RepositoryError> {
        let mut source = SourceMap::new();
        let mut pending_directories = vec![root.to_path_buf()];
        let mut visited_directories = HashSet::new();

        while let Some(directory) = pending_directories.pop() {
            // skip already visited directories
            if !visited_directories.insert(directory.clone()) {
                continue;
            }

            let entries = self.fs.read_dir(&directory).map_err(|error| {
                RepositoryError::ImportFileSystem {
                    operation: "read_dir",
                    path: directory.clone(),
                    message: error.to_string(),
                }
            })?;

            for entry in entries {
                let metadata = self.fs.metadata(&entry).map_err(|error| {
                    RepositoryError::ImportFileSystem {
                        operation: "metadata",
                        path: entry.clone(),
                        message: error.to_string(),
                    }
                })?;

                // recurse into workspace directories
                if metadata.is_directory {
                    if self.should_seed_workspace_directory(&entry) {
                        pending_directories.push(entry);
                    }

                    continue;
                }

                // skip non-file entries
                if !metadata.is_file {
                    continue;
                }

                // seed one tracked workspace file
                if self.should_seed_workspace_file(&entry) {
                    self.seed_workspace_file(&mut source, &entry)?;
                }
            }
        }

        Ok(source)
    }

    /// Load one workspace file payload from the attached file system.
    pub fn load_workspace_file_content(&self, path: &Path) -> Result<FileContent, RepositoryError> {
        let file_type = FileType::from_path_or_unknown(path);

        if file_type.is_binary() {
            let content =
                self.fs
                    .read(path)
                    .map_err(|error| RepositoryError::ImportFileSystem {
                        operation: "read",
                        path: path.to_path_buf(),
                        message: error.to_string(),
                    })?;

            return Ok(FileContent::Binary { content });
        }

        let content =
            self.fs
                .read_to_string(path)
                .map_err(|error| RepositoryError::ImportFileSystem {
                    operation: "read_to_string",
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })?;

        Ok(FileContent::Text { content })
    }

    /// Apply one explicit seeded source map to one ref.
    pub(crate) fn apply_seeded_source(
        &self,
        reference: &Ref,
        seeded_source: SourceMap,
    ) -> Result<Revision, RepositoryError> {
        let base_revision_id = self.current(reference)?;
        let base_revision = self.revision(base_revision_id)?;
        let mut source = base_revision.source.as_ref().clone();

        for (file_id, content_id) in seeded_source {
            source.insert(file_id, content_id);
        }

        let revision = Arc::new(self.build_revision_data(smallvec![base_revision_id], source));
        let revision_id = revision.revision();

        self.revisions
            .entry(revision_id)
            .or_insert_with(|| Arc::clone(&revision));
        self.refs.insert(reference.clone(), revision_id);

        Ok(revision_id)
    }

    /// Build one full revision snapshot from one source map.
    fn build_revision_data(
        &self,
        parents: smallvec::SmallVec<[Revision; 2]>,
        source: SourceMap,
    ) -> RevisionData {
        RevisionData::new(parents, Arc::new(source))
    }

    /// Derive package views from one source map.
    pub(crate) fn derive_packages(&self, source: &SourceMap) -> OrdMap<PackageId, Package> {
        let mut physical_roots: HashSet<PathBuf> = HashSet::new();

        // package roots
        for file_id in source.keys() {
            let Some(FileOrigin::Workspace { path }) = self.file_origin_by_file_id(*file_id) else {
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
        for file_id in source.keys() {
            let Some(origin) = self.file_origin_by_file_id(*file_id) else {
                continue;
            };
            let Some(mut package) = self.package_for_file_origin(&physical_roots, &origin) else {
                continue;
            };
            let package_id = package.id;

            if let Some(package_path) = package.path.as_ref()
                && let FileOrigin::Workspace { path } = &origin
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

    /// Derive module views from one source map.
    pub(crate) fn derive_modules(
        &self,
        source: &SourceMap,
        packages: &OrdMap<PackageId, Package>,
    ) -> OrdMap<ModuleId, Module> {
        let mut modules = OrdMap::new();

        for file_id in source.keys() {
            let Some(origin) = self.file_origin_by_file_id(*file_id) else {
                continue;
            };

            if matches!(origin, FileOrigin::Root) {
                let uri = Uri::from_string("<root>");
                let module = Module::blank(
                    self.root_module_id(),
                    *file_id,
                    uri,
                    None,
                    PackageId::EPHEMERAL,
                    LanguageType::Destack,
                    Loader::Destack,
                    ModuleSource::User,
                );

                modules.insert(module.id, module);
                continue;
            }

            if let Some(module) = self.builtin_module(*file_id, &origin) {
                modules.insert(module.id, module);
                continue;
            }

            let FileOrigin::Workspace { path } = origin else {
                continue;
            };
            let path = path.to_path_buf();
            let file_type = FileType::from_path_or_unknown(&path);
            if !self.is_module_file(&path, file_type) {
                continue;
            }

            let Some(package) = self.package_for_path(packages, &path) else {
                continue;
            };

            let language_type = LanguageType::from(file_type);
            let loader = Loader::from_file_type(file_type);
            let module_id =
                ModuleId::from_path_with_loader(package.id, &path, package.path.as_deref(), None);
            let module = Module::blank(
                module_id,
                *file_id,
                Uri::from_path(&path),
                Some(path),
                package.id,
                language_type,
                loader,
                ModuleSource::User,
            );

            modules.insert(module_id, module);
        }

        modules
    }

    /// Return the package root and kind for one workspace file path.
    fn package_root_for_path(
        &self,
        physical_roots: &HashSet<PathBuf>,
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
    fn package_for_file_origin(
        &self,
        physical_roots: &HashSet<PathBuf>,
        origin: &FileOrigin,
    ) -> Option<Package> {
        match origin {
            FileOrigin::Root => Some(Package {
                id: PackageId::EPHEMERAL,
                kind: PackageKind::Ephemeral,
                uri: Uri::from_string("<root>"),
                path: None,
                name: None,
                version: None,
                manifest: None,
                config: None,
                tsconfig_file_id: None,
                targets: Default::default(),
            }),
            FileOrigin::Builtin { .. } => Some(Package {
                id: BUILTIN_PACKAGE_ID,
                kind: PackageKind::Builtin,
                uri: Uri::from_string("builtin://"),
                path: None,
                name: None,
                version: None,
                manifest: None,
                config: None,
                tsconfig_file_id: None,
                targets: Default::default(),
            }),
            FileOrigin::Workspace { path } => {
                let (kind, package_root) = self.package_root_for_path(physical_roots, path);
                Some(Package {
                    id: self.package_id_for_root(kind, &package_root),
                    kind,
                    uri: Uri::from_path(&package_root),
                    path: Some(package_root),
                    name: None,
                    version: None,
                    manifest: None,
                    config: None,
                    tsconfig_file_id: None,
                    targets: Default::default(),
                })
            }
        }
    }

    /// Return whether one workspace file should materialize as a module.
    fn is_module_file(&self, path: &Path, file_type: FileType) -> bool {
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };

        if matches!(file_name, "package.json" | "destack.json" | "tsconfig.json") {
            return false;
        }

        file_type.is_code() || file_type.is_data() || file_type.is_text() || file_type.is_binary()
    }

    /// Return the nearest package entry for one path.
    fn package_for_path<'a>(
        &self,
        packages: &'a OrdMap<PackageId, Package>,
        path: &Path,
    ) -> Option<&'a Package> {
        packages
            .values()
            .filter_map(|package| {
                let package_path = package.path.as_ref()?;
                if !path.starts_with(package_path) {
                    return None;
                }

                Some((package_path.as_os_str().len(), package))
            })
            .max_by_key(|(package_length, _)| *package_length)
            .map(|(_, package)| package)
    }

    /// Return true when the seed walk should descend into one directory.
    fn should_seed_workspace_directory(&self, path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return true;
        };

        !matches!(name, ".destack" | ".git" | "node_modules" | "target")
    }

    /// Return true when one path should be present in the workspace base revision.
    fn should_seed_workspace_file(&self, path: &Path) -> bool {
        let file_type = FileType::from_path_or_unknown(path);

        file_type.is_code() || file_type.is_data() || file_type.is_text() || file_type.is_binary()
    }

    /// Seed one file into one workspace source snapshot.
    fn seed_workspace_file(
        &self,
        source: &mut SourceMap,
        path: &Path,
    ) -> Result<(), RepositoryError> {
        let file_content = self.load_workspace_file_content(path)?;
        let file_id = self.intern_workspace_file_id(path);
        let content_id = self.contents.intern(file_content);

        source.insert(file_id, content_id);

        Ok(())
    }

    /// Insert one explicit seeded source binding.
    pub(crate) fn insert_seeded_source(
        &self,
        source: &mut SourceMap,
        logical_path: &str,
        content: FileContent,
    ) {
        let file_id = if logical_path == "<root>" {
            self.intern_root_file_id(logical_path)
        } else if let Some((module_path, source_kind)) =
            self.builtins.module_origin_by_logical_path(logical_path)
        {
            self.intern_builtin_file_id(logical_path, &module_path, source_kind)
        } else {
            panic!("seeded source must have an explicit origin: {logical_path}")
        };
        let content_id = self.contents.intern(content);

        source.insert(file_id, content_id);
    }

    /// Validate one workspace logical path.
    fn validate_workspace_logical_path(&self, logical_path: &str) -> Result<(), RepositoryError> {
        if logical_path == "<root>" {
            return Err(RepositoryError::InvalidEditPath {
                path: logical_path.to_string(),
                message: "root source is repository seeded".to_string(),
            });
        }

        if logical_path.starts_with("builtin://") {
            return Err(RepositoryError::InvalidEditPath {
                path: logical_path.to_string(),
                message: "builtin source is repository seeded".to_string(),
            });
        }

        if logical_path.contains("://") {
            return Err(RepositoryError::InvalidEditPath {
                path: logical_path.to_string(),
                message: "non workspace source is not writable through repository edits"
                    .to_string(),
            });
        }

        Ok(())
    }

    /// Return one file id for one validated workspace logical path.
    fn file_id_for_workspace_logical_path(
        &self,
        logical_path: &str,
    ) -> Result<FileId, RepositoryError> {
        self.validate_workspace_logical_path(logical_path)?;

        Ok(self.file_id_for_logical_path(logical_path))
    }

    fn apply_change(
        &self,
        mut source: SourceMap,
        change: Change,
    ) -> Result<SourceMap, RepositoryError> {
        for edit in change.edits {
            match edit {
                // write the requested file payload
                Edit::AddFile {
                    logical_path,
                    content,
                } => {
                    let file_id = self.file_id_for_workspace_logical_path(&logical_path)?;
                    if source.contains_key(&file_id) {
                        return Err(RepositoryError::FileAlreadyExists { path: logical_path });
                    }

                    let file_id = self.intern_workspace_logical_file_id(&logical_path);
                    let content = self.contents.intern(content);
                    source.insert(file_id, content);
                }

                // set the requested file payload
                Edit::SetFile {
                    logical_path,
                    content,
                } => {
                    let file_id = self.intern_workspace_logical_file_id(&logical_path);
                    let content = self.contents.intern(content);
                    source.insert(file_id, content);
                }

                // remove the requested file payload
                Edit::RemoveFile { logical_path } => {
                    let file_id = self.file_id_for_workspace_logical_path(&logical_path)?;
                    if !source.contains_key(&file_id) {
                        return Err(RepositoryError::MissingFile { path: logical_path });
                    }

                    source.remove(&file_id);
                }

                // move one existing file binding
                Edit::MoveFile { from, to } => {
                    if from == to {
                        continue;
                    }

                    let from_file_id = self.file_id_for_workspace_logical_path(&from)?;
                    if !source.contains_key(&from_file_id) {
                        return Err(RepositoryError::MissingFile { path: from });
                    }
                    let Some(from_file) = source.get(&from_file_id).cloned() else {
                        return Err(RepositoryError::MissingFile { path: from });
                    };
                    let to_file_id = self.file_id_for_workspace_logical_path(&to)?;
                    if source.contains_key(&to_file_id) {
                        return Err(RepositoryError::FileAlreadyExists { path: to });
                    }

                    source.remove(&from_file_id);
                    let to_file_id = self.intern_workspace_logical_file_id(&to);

                    source.insert(to_file_id, from_file);
                }
            }
        }

        Ok(source)
    }

    /// Return one builtin module entry for one file origin when applicable.
    fn builtin_module(&self, file_id: FileId, origin: &FileOrigin) -> Option<Module> {
        let FileOrigin::Builtin {
            module_path,
            source,
        } = origin
        else {
            return None;
        };

        let file_type = FileType::from_path_or_unknown(Path::new(module_path));
        let language_type = LanguageType::from(file_type);
        let loader = Loader::from_file_type(file_type);
        let module_id = ModuleId::from_relative_path(BUILTIN_PACKAGE_ID, Path::new(module_path));

        Some(Module::blank(
            module_id,
            file_id,
            Uri::from_string(
                self.logical_path_by_file_id(file_id)
                    .unwrap_or_else(|| Arc::<str>::from(format!("builtin://{module_path}")))
                    .as_ref(),
            ),
            None,
            BUILTIN_PACKAGE_ID,
            language_type,
            loader,
            *source,
        ))
    }
}
