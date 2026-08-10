use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::str;
use std::sync::Arc;

use destack_artifact::BuildId;
use destack_source as source;
use destack_source::{
    File, FileId, FileMetadata, FilePatch, FileSystem, FileType, MemoryFileSystem, Patch, Span,
    TextPatch, Uri, apply_file_patch, matches,
};

use crate::{
    BlobStore, Dependency, DestackFile, DestackLayout, DestackLayoutOverride, Edit, Environment,
    Host, MemoryBlobStore, Ref, Repository, RepositoryError, Revision, Settings,
};

/// Open one repository after discovering the source root from one path.
pub fn open_repository_from_fs(
    path: PathBuf,
    file_system: Arc<dyn FileSystem>,
    environment: Environment,
    settings: Settings,
    layout_override: DestackLayoutOverride,
) -> Result<Repository, RepositoryError> {
    let build_id = BuildId::current().map_err(|error| RepositoryError::ArtifactStore {
        message: format!("failed to identify Destack build: {error}"),
    })?;
    let host = Host::new(build_id, environment, file_system);

    open_repository(path, host, settings, layout_override)
}

/// Open one repository from one in-memory source.
pub fn open_repository_from_memory(
    root: PathBuf,
    edits: Vec<source::Edit>,
    environment: Environment,
    settings: Settings,
    layout_override: DestackLayoutOverride,
) -> Result<Repository, RepositoryError> {
    let file_system = Arc::new(MemoryFileSystem::new());
    apply_edits(file_system.as_ref(), &root, edits)?;

    // keep memory repositories fully in memory
    let blob_store: Arc<dyn BlobStore> = Arc::new(MemoryBlobStore::new());
    let build_id = BuildId::current().map_err(|error| RepositoryError::ArtifactStore {
        message: format!("failed to identify Destack build: {error}"),
    })?;
    let host = Host::new(build_id, environment, file_system).with_blob_store(blob_store);

    open_repository(root, host, settings, layout_override)
}

/// Open one repository from explicit host capabilities.
pub fn open_repository(
    path: PathBuf,
    host: Host,
    settings: Settings,
    layout_override: DestackLayoutOverride,
) -> Result<Repository, RepositoryError> {
    let root = PathBuf::from(SourceRoot::discover(host.files().as_ref(), &path)?);
    let environment = host.environment();
    let cwd = environment.cwd.as_deref().unwrap_or(&path);
    let layout = DestackLayout::resolve(&root, cwd, environment, &settings, &layout_override, None);

    // create repository at the selected source root
    let repository = Repository::new(root.clone(), host, settings, layout);
    let root_ref = Ref::for_root(&root);
    let base_revision = repository.current(&root_ref)?;

    // read the complete source tree
    let edits = repository.scan(&root, base_revision)?;
    let revision = repository.edit(base_revision, edits)?.after;

    repository.set_ref(&root_ref, revision)?;

    Ok(repository)
}

/// A source root discovered from one filesystem path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceRoot {
    /// A root declared by a `destack.json` manifest.
    Declared(PathBuf),
    /// A root implied by a source path outside a declared package.
    Implicit(PathBuf),
}

impl SourceRoot {
    /// Discover the nearest source root for one filesystem path.
    pub fn discover(file_system: &dyn FileSystem, path: &Path) -> Result<Self, RepositoryError> {
        let metadata = file_system
            .metadata(path)
            .map_err(|error| RepositoryError::FileSystem {
                operation: "metadata",
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;

        // normalize file inputs to their containing directory
        let directory = if metadata.is_file {
            path.parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| path.to_path_buf())
        } else {
            path.to_path_buf()
        };
        let mut current = directory.clone();

        // walk up directories looking for a declared source root
        loop {
            if DestackFile::read(file_system, &current)?.is_some() {
                return Ok(Self::Declared(current));
            }

            let Some(parent) = current.parent() else {
                break;
            };
            current = parent.to_path_buf();
        }

        Ok(Self::Implicit(directory))
    }
}

impl From<SourceRoot> for PathBuf {
    /// Consume one discovered source root.
    fn from(root: SourceRoot) -> Self {
        match root {
            SourceRoot::Declared(path) | SourceRoot::Implicit(path) => path,
        }
    }
}

impl Repository {
    /// Return edits needed to import one physical file and its package state.
    pub fn scan_file(
        &self,
        root: &Path,
        base: Revision,
        logical_path: PathBuf,
    ) -> Result<Vec<Edit>, RepositoryError> {
        Scan::new(self, root, base).load(logical_path)
    }

    /// Return exact edits for physical paths that may have changed.
    pub fn scan_paths(
        &self,
        root: &Path,
        base: Revision,
        logical_paths: Vec<PathBuf>,
    ) -> Result<Vec<Edit>, RepositoryError> {
        Scan::new(self, root, base).reconcile(logical_paths)
    }

    /// Return edits needed to match one complete physical source tree.
    pub fn scan(&self, root: &Path, base: Revision) -> Result<Vec<Edit>, RepositoryError> {
        Scan::new(self, root, base).scan()
    }
}

/// One physical repository scan.
#[derive(Debug)]
struct Scan<'a> {
    /// The repository compared with physical state.
    repository: &'a Repository,
    /// The base repository revision.
    base: Revision,
    /// The physical repository root.
    root: &'a Path,
    /// Repository edits built by this scan.
    edits: Vec<Edit>,
    /// File ids seen during this scan.
    seen_file_ids: HashSet<FileId>,
    /// Package roots still to scan.
    pending_packages: Vec<(PathBuf, DestackFile)>,
    /// Package roots already queued.
    queued_package_roots: HashSet<PathBuf>,
}

impl<'a> Scan<'a> {
    /// Create one physical repository scan.
    fn new(repository: &'a Repository, root: &'a Path, base: Revision) -> Self {
        Self {
            repository,
            base,
            root,
            edits: Vec::new(),
            seen_file_ids: HashSet::new(),
            pending_packages: Vec::new(),
            queued_package_roots: HashSet::new(),
        }
    }

    /// Return whether a repository scan imports one path.
    fn tracks_path(path: &Path) -> bool {
        // import source manifests directly
        if Self::is_destack_config_path(path) {
            return true;
        }

        // import recognized source file classes
        let Some(file_type) = FileType::from_path(path) else {
            return false;
        };

        file_type.is_code() || file_type.is_data() || file_type.is_text() || file_type.is_binary()
    }

    /// Return whether one path is a destack manifest.
    fn is_destack_config_path(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "destack.json")
    }

    /// Read one `destack.json` from a physical directory when present.
    fn read_config(&self, root: &Path) -> Result<Option<DestackFile>, RepositoryError> {
        let file_system = self.repository.file_system();

        DestackFile::read(file_system.as_ref(), root)
    }

    /// Return the nearest enclosing package within this source root.
    fn package(&self, directory: &Path) -> Result<Option<DestackFile>, RepositoryError> {
        let mut current = Some(directory);

        // walk toward the source root until a package manifest is found
        while let Some(candidate) = current {
            if let Some(config) = self.read_config(candidate)? {
                return Ok(Some(config));
            }
            if candidate == self.root {
                break;
            }
            current = candidate
                .parent()
                .filter(|parent| parent.starts_with(self.root));
        }

        Ok(None)
    }

    /// Return whether one physical path belongs to its effective package source set.
    fn includes(&self, path: &Path) -> Result<bool, RepositoryError> {
        let directory = path.parent().unwrap_or(self.root);
        let Some(config) = self.package(directory)? else {
            return Ok(Self::tracks_path(path));
        };
        let package_path = self.package_path(&config.directory, path)?;

        Ok(config.includes_source(&package_path))
    }

    /// Return whether effective package configuration excludes one physical path.
    fn excludes(&self, path: &Path) -> Result<bool, RepositoryError> {
        let directory = path.parent().unwrap_or(self.root);
        let Some(config) = self.package(directory)? else {
            return Ok(false);
        };
        let package_path = self.package_path(&config.directory, path)?;

        Ok(config.excludes_source(&package_path))
    }

    /// Queue package roots selected by one workspace manifest.
    fn queue_workspace_packages(&mut self, config: DestackFile) -> Result<(), RepositoryError> {
        let workspace_package_patterns = config.workspace_packages().map(<[_]>::to_vec);

        // import the workspace manifest first
        self.import_file(&config.path)?;

        // queue one package at the source root
        if workspace_package_patterns.is_none() {
            self.queue_package(self.root.to_path_buf(), config);
        }
        // queue declared workspace packages
        else if let Some(patterns) = workspace_package_patterns.as_deref() {
            for config_path in self.find_destack_config_paths(&config)? {
                let Some(package_root) = config_path.parent().map(Path::to_path_buf) else {
                    return Err(RepositoryError::InvalidConfigFile {
                        path: config_path,
                        message: "destack.json must have a parent directory".to_string(),
                    });
                };
                let mut is_workspace_package = false;
                for pattern in patterns {
                    if self.matches_workspace_pattern(&package_root, pattern)? {
                        is_workspace_package = true;
                        break;
                    }
                }
                if !is_workspace_package {
                    continue;
                }

                let Some(config) = self.read_config(&package_root)? else {
                    continue;
                };
                self.queue_package(package_root, config);
            }
        }

        Ok(())
    }

    /// Queue one package root when it has not already been queued.
    fn queue_package(&mut self, package_root: PathBuf, config: DestackFile) {
        if self.queued_package_roots.insert(package_root.clone()) {
            self.pending_packages.push((package_root, config));
        }
    }

    /// Find `destack.json` files below the source root.
    fn find_destack_config_paths(
        &self,
        config: &DestackFile,
    ) -> Result<Vec<PathBuf>, RepositoryError> {
        let mut configs = Vec::new();
        let mut pending = vec![self.root.to_path_buf()];
        let mut visited = HashSet::new();

        // walk directories by metadata only
        while let Some(directory) = pending.pop() {
            if !visited.insert(directory.clone()) {
                continue;
            }

            for path in self.read_directory(&directory)? {
                let metadata = self.metadata(&path)?;

                // collect manifest files
                if metadata.is_file && Self::is_destack_config_path(&path) {
                    configs.push(path);
                }
                // descend into source directories
                else if metadata.is_directory {
                    let package_path = self.package_path(self.root, &path)?;
                    if !config.excludes_source(&package_path) {
                        pending.push(path);
                    }
                }
            }
        }

        configs.sort();
        configs.dedup();

        Ok(configs)
    }

    /// Scan source files for one package root.
    fn scan_package_sources(
        &mut self,
        package_root: &Path,
        config: &DestackFile,
    ) -> Result<(), RepositoryError> {
        // import the package manifest
        self.import_file(&config.path)?;

        // add listed files
        for file in &config.files {
            let path = package_root.join(file);
            let package_path = self.package_path(package_root, &path)?;
            if config.excludes_source(&package_path) {
                continue;
            }

            let metadata = self.metadata(&path)?;
            if metadata.is_file {
                self.import_file(&path)?;
            }
        }

        self.scan_include_paths(package_root, config)
    }

    /// Scan included source files under one package root.
    fn scan_include_paths(
        &mut self,
        package_root: &Path,
        config: &DestackFile,
    ) -> Result<(), RepositoryError> {
        let mut pending = vec![package_root.to_path_buf()];
        let mut visited = HashSet::new();

        // walk package files by metadata only
        while let Some(directory) = pending.pop() {
            if !visited.insert(directory.clone()) {
                continue;
            }

            for path in self.read_directory(&directory)? {
                let metadata = self.metadata(&path)?;

                // descend into directories that remain inside the source set
                if metadata.is_directory {
                    let package_path = self.package_path(package_root, &path)?;
                    if !config.excludes_source(&package_path) {
                        pending.push(path);
                    }
                    continue;
                }

                // collect matching files
                let package_path = self.package_path(package_root, &path)?;
                if metadata.is_file && config.includes_source(&package_path) {
                    self.import_file(&path)?;
                }
            }
        }

        Ok(())
    }

    /// Scan trackable files directly within one implicit source directory.
    fn scan_directory(&mut self, directory: &Path) -> Result<(), RepositoryError> {
        for path in self.read_directory(directory)? {
            if !Self::tracks_path(&path) {
                continue;
            }
            let metadata = self.metadata(&path)?;
            if metadata.is_file {
                self.import_file(&path)?;
            }
        }

        Ok(())
    }

    /// Queue local path dependency roots declared by one package config.
    fn queue_path_dependencies(
        &mut self,
        package_root: &Path,
        config: &DestackFile,
    ) -> Result<(), RepositoryError> {
        let dependencies = config.dependencies.iter().chain(
            config
                .conditional_dependencies
                .iter()
                .flat_map(|conditional| conditional.dependencies.iter()),
        );

        // add filesystem path dependencies
        for (name, dependency) in dependencies {
            let Dependency::Path { path } = dependency else {
                continue;
            };
            let dependency_root = package_root.join(path);
            let dependency_root = self
                .repository
                .file_system()
                .canonicalize(&dependency_root)
                .map_err(|error| RepositoryError::FileSystem {
                    operation: "canonicalize",
                    path: dependency_root,
                    message: error.to_string(),
                })?;
            let Some(config) = self.read_config(&dependency_root)? else {
                continue;
            };

            // roots escaping the workspace mount under their dependency name
            if !dependency_root.starts_with(self.root) {
                self.repository.add_mount(name, dependency_root.clone())?;
            }

            self.queue_package(dependency_root, config);
        }

        Ok(())
    }

    /// Import one physical file path into this scan.
    fn import_file(&mut self, path: &Path) -> Result<(), RepositoryError> {
        let logical_path = self.repository.logical_path(path);

        self.import_file_with_logical_path(path, Path::new(&logical_path))
    }

    /// Import one physical file with one logical path into this scan.
    fn import_file_with_logical_path(
        &mut self,
        path: &Path,
        logical_path: &Path,
    ) -> Result<(), RepositoryError> {
        let logical_path = self.path_text(logical_path);
        let file_id = FileId::from_logical_str(&logical_path);
        let mut input = self.repository.file_system().open(path).map_err(|error| {
            RepositoryError::FileSystem {
                operation: "open",
                path: path.to_path_buf(),
                message: error.to_string(),
            }
        })?;
        let incoming = self
            .repository
            .blob_store()
            .put(input.as_mut())
            .map_err(|error| RepositoryError::Blob {
                message: error.to_string(),
            })?;

        // require text files to contain valid UTF-8 before publishing their binding
        if !FileType::from_path_or_unknown(path).is_binary() {
            let memory = self
                .repository
                .blob_store()
                .open(incoming)
                .map_err(|error| RepositoryError::Blob {
                    message: error.to_string(),
                })?;
            str::from_utf8(memory.bytes()).map_err(|error| RepositoryError::InvalidFile {
                file: file_id,
                message: error.to_string(),
            })?;
        }

        self.seen_file_ids.insert(file_id);
        let current = self.repository.file_blob(self.base, file_id)?;

        // record changed files only
        if current != Some(incoming) {
            self.edits.push(Edit::SetFile {
                logical_path,
                blob: incoming,
            });
        }

        Ok(())
    }

    /// Return true when one workspace package pattern matches one root.
    fn matches_workspace_pattern(
        &self,
        package_root: &Path,
        pattern: &str,
    ) -> Result<bool, RepositoryError> {
        let package_path = package_root
            .strip_prefix(self.root)
            .map_err(|_| RepositoryError::PathOutsideRoot {
                path: package_root.to_path_buf(),
                root: self.root.to_path_buf(),
            })?
            .to_string_lossy()
            .replace('\\', "/");
        let package_path = if package_path.is_empty() {
            "."
        } else {
            package_path.as_str()
        };

        // match either the package directory or its manifest
        let config_path = if package_path == "." {
            "destack.json".to_string()
        } else {
            format!("{package_path}/destack.json")
        };

        Ok(matches(pattern.as_bytes(), 0, package_path.as_bytes(), 0)
            || matches(pattern.as_bytes(), 0, config_path.as_bytes(), 0))
    }

    /// Return one package-relative path as normalized text.
    fn package_path(&self, package_root: &Path, path: &Path) -> Result<String, RepositoryError> {
        let package_path =
            path.strip_prefix(package_root)
                .map_err(|_| RepositoryError::PathOutsideRoot {
                    path: path.to_path_buf(),
                    root: package_root.to_path_buf(),
                })?;

        Ok(self.path_text(package_path))
    }

    /// Return one path as normalized logical text.
    fn path_text(&self, path: impl AsRef<Path>) -> String {
        path.as_ref().to_string_lossy().replace('\\', "/")
    }

    /// Read child paths for one directory.
    fn read_directory(&self, directory: &Path) -> Result<Vec<PathBuf>, RepositoryError> {
        let mut paths = self
            .repository
            .file_system()
            .read_dir(directory)
            .map_err(|error| RepositoryError::FileSystem {
                operation: "read_dir",
                path: directory.to_path_buf(),
                message: error.to_string(),
            })?;
        paths.sort();

        Ok(paths)
    }

    /// Read metadata for one path.
    fn metadata(&self, path: &Path) -> Result<FileMetadata, RepositoryError> {
        self.repository
            .file_system()
            .metadata(path)
            .map_err(|error| RepositoryError::FileSystem {
                operation: "metadata",
                path: path.to_path_buf(),
                message: error.to_string(),
            })
    }
}

impl Scan<'_> {
    /// Return repository edits needed to load one explicit logical source path.
    fn load(mut self, logical_path: PathBuf) -> Result<Vec<Edit>, RepositoryError> {
        if logical_path.is_absolute() {
            return Err(RepositoryError::InvalidEditPath {
                path: self.path_text(&logical_path),
                message: "repository scan paths must be relative".to_string(),
            });
        }
        let path = self.root.join(&logical_path);

        // return an empty edit batch when the requested file does not exist
        let metadata = match self.repository.file_system().metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(RepositoryError::FileSystem {
                    operation: "metadata",
                    path,
                    message: error.to_string(),
                });
            }
        };
        if !metadata.is_file {
            return Ok(Vec::new());
        }

        // import the explicit file and the package state needed to resolve it
        self.import_file_with_logical_path(&path, &logical_path)?;
        let directory = path.parent().unwrap_or(self.root);
        if let Some(config) = self.package(directory)? {
            let package_root = config.directory.clone();
            self.queue_package(package_root, config);
            self.scan_queued_packages()?;
        } else {
            self.scan_directory(directory)?;
        }

        Ok(self.edits)
    }

    /// Return exact repository edits for changed logical paths.
    fn reconcile(mut self, mut logical_paths: Vec<PathBuf>) -> Result<Vec<Edit>, RepositoryError> {
        // reconcile each logical path once in stable order
        logical_paths.sort();
        logical_paths.dedup();

        // classify paths before mutating the pending edit batch
        let mut paths = Vec::with_capacity(logical_paths.len());
        for logical_path in logical_paths {
            if logical_path.is_absolute() {
                return Err(RepositoryError::InvalidEditPath {
                    path: self.path_text(&logical_path),
                    message: "repository scan paths must be relative".to_string(),
                });
            }
            let path = self.root.join(&logical_path);

            // manifests can change every package source binding
            if Self::is_destack_config_path(&logical_path) {
                return self.scan();
            }
            match self.repository.file_system().metadata(&path) {
                Ok(metadata) if metadata.is_file => {
                    let file_id = FileId::from_logical_path(&logical_path);
                    let blob = self.repository.file_blob(self.base, file_id)?;
                    if blob.is_some() || self.includes(&path)? {
                        paths.push((logical_path, path, true));
                    }
                }
                Ok(metadata) if metadata.is_directory => {
                    if !self.excludes(&path)? {
                        return self.scan();
                    }
                }
                Ok(_) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => {
                    paths.push((logical_path, path, false));
                }
                Err(error) => {
                    return Err(RepositoryError::FileSystem {
                        operation: "metadata",
                        path,
                        message: error.to_string(),
                    });
                }
            }
        }

        // reconcile current filesystem truth against the immutable base
        for (logical_path, path, is_file) in paths {
            if is_file {
                self.import_file_with_logical_path(&path, &logical_path)?;
            } else {
                self.remove_path(&logical_path)?;
            }
        }

        Ok(self.edits)
    }

    /// Return repository edits for the complete source state.
    fn scan(mut self) -> Result<Vec<Edit>, RepositoryError> {
        self.scan_root()?;
        self.remove_missing_files()?;

        Ok(self.edits)
    }

    /// Scan the complete source state beneath this physical root.
    fn scan_root(&mut self) -> Result<(), RepositoryError> {
        // scan declared package membership
        if let Some(workspace_config) = self.read_config(self.root)? {
            self.queue_workspace_packages(workspace_config)?;

            self.scan_queued_packages()
        }
        // scan trackable siblings in one implicit source root
        else {
            let root = self.root.to_path_buf();

            self.scan_directory(&root)
        }
    }

    /// Expand queued package sources and their local dependencies.
    fn scan_queued_packages(&mut self) -> Result<(), RepositoryError> {
        let mut index = 0;
        while index < self.pending_packages.len() {
            let (package_root, config) = self.pending_packages[index].clone();
            index += 1;

            self.scan_package_sources(&package_root, &config)?;
            self.queue_path_dependencies(&package_root, &config)?;
        }

        Ok(())
    }

    /// Remove repository files missing from a complete source scan.
    fn remove_missing_files(&mut self) -> Result<(), RepositoryError> {
        for (file_id, logical_path) in self.repository.editable_file_logical_paths(self.base)? {
            if !self.seen_file_ids.contains(&file_id) {
                let logical_path = self.repository.string_pool().get(logical_path);
                self.edits.push(Edit::remove_file(logical_path));
            }
        }

        Ok(())
    }

    /// Remove one missing logical path and its tracked descendants.
    fn remove_path(&mut self, logical_path: &Path) -> Result<(), RepositoryError> {
        let logical_path = self.path_text(logical_path);
        let descendant_prefix = format!("{logical_path}/");

        // remove exact files and files beneath a missing directory
        for (file_id, stored_path) in self.repository.editable_file_logical_paths(self.base)? {
            let stored_path = self.repository.string_pool().get(stored_path);
            let is_affected =
                stored_path == logical_path || stored_path.starts_with(&descendant_prefix);
            if is_affected && self.seen_file_ids.insert(file_id) {
                self.edits.push(Edit::remove_file(stored_path));
            }
        }

        Ok(())
    }
}

/// Apply source edits to one filesystem root.
fn apply_edits(
    file_system: &dyn FileSystem,
    root: &Path,
    edits: Vec<source::Edit>,
) -> Result<(), RepositoryError> {
    file_system
        .create_dir_all(root)
        .map_err(|error| RepositoryError::FileSystem {
            operation: "create_dir_all",
            path: root.to_path_buf(),
            message: error.to_string(),
        })?;

    // apply edits in input order
    for edit in edits {
        apply_edit(file_system, root, edit)?;
    }

    Ok(())
}

/// Apply one source edit to one filesystem root.
fn apply_edit(
    file_system: &dyn FileSystem,
    root: &Path,
    edit: source::Edit,
) -> Result<(), RepositoryError> {
    match edit {
        source::Edit::SetText { path, text } => {
            let path = root.join(path);

            write_text(file_system, &path, &text)
        }
        source::Edit::SetBytes { path, bytes } => {
            let path = root.join(path);

            write_bytes(file_system, &path, &bytes)
        }
        source::Edit::EditText { path, patches } => {
            let full_path = root.join(&path);

            patch_text(file_system, &full_path, &path, patches)
        }
        source::Edit::Remove { path } => {
            let path = root.join(path);

            remove_path(file_system, &path)
        }
        source::Edit::Move { from, to } => {
            let from = root.join(from);
            let to = root.join(to);

            move_path(file_system, &from, &to)
        }
    }
}

/// Write one text file.
fn write_text(
    file_system: &dyn FileSystem,
    path: &Path,
    text: &str,
) -> Result<(), RepositoryError> {
    create_parent_directory(file_system, path)?;

    file_system
        .write_string(path, text)
        .map_err(|error| RepositoryError::FileSystem {
            operation: "write_string",
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

    Ok(())
}

/// Write one binary file.
fn write_bytes(
    file_system: &dyn FileSystem,
    path: &Path,
    bytes: &[u8],
) -> Result<(), RepositoryError> {
    create_parent_directory(file_system, path)?;

    file_system
        .write(path, bytes)
        .map_err(|error| RepositoryError::FileSystem {
            operation: "write",
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

    Ok(())
}

/// Apply text patches to one text file.
fn patch_text(
    file_system: &dyn FileSystem,
    path: &Path,
    logical_path: &Path,
    patches: Vec<TextPatch>,
) -> Result<(), RepositoryError> {
    let text = file_system
        .read_to_string(path)
        .map_err(|error| RepositoryError::FileSystem {
            operation: "read_to_string",
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

    // reject files outside source coordinates
    let length = text.len();
    if length > File::MAX_BYTES {
        return Err(RepositoryError::InvalidFile {
            file: FileId::from_logical_path(logical_path),
            message: format!("File contains {length} bytes beyond the source limit"),
        });
    }

    // build the source file for patch application
    let file_id = FileId::from_logical_path(logical_path);
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return Err(RepositoryError::InvalidEditPath {
            path: logical_path.to_string_lossy().into_owned(),
            message: "text edit path must name a UTF-8 file".to_string(),
        });
    };
    let name = name.to_string();
    let file = File::from_text(
        file_id,
        name,
        Uri::from_file_path(path),
        Some(path.to_path_buf()),
        FileType::from_path_or_unknown(path),
        text,
    )
    .map_err(|error| RepositoryError::InvalidFile {
        file: file_id,
        message: error.to_string(),
    })?;

    // lower path level text patches into source patches
    let patches = patches
        .into_iter()
        .map(|patch| {
            Patch::replace(
                Span::new(file_id, patch.range.start, patch.range.end),
                patch.text,
            )
        })
        .collect();
    let file_patch = FilePatch::with_patches(file_id, patches);
    let text =
        apply_file_patch(&file, &file_patch).map_err(|error| RepositoryError::FileSystem {
            operation: "apply_file_patch",
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

    write_text(file_system, path, &text)
}

/// Remove one path.
fn remove_path(file_system: &dyn FileSystem, path: &Path) -> Result<(), RepositoryError> {
    file_system
        .remove_path(path)
        .map_err(|error| RepositoryError::FileSystem {
            operation: "remove_path",
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

    Ok(())
}

/// Move one file path.
fn move_path(file_system: &dyn FileSystem, from: &Path, to: &Path) -> Result<(), RepositoryError> {
    let bytes = file_system
        .read(from)
        .map_err(|error| RepositoryError::FileSystem {
            operation: "read",
            path: from.to_path_buf(),
            message: error.to_string(),
        })?;

    write_bytes(file_system, to, &bytes)?;
    remove_path(file_system, from)
}

/// Create the parent directory for one file path.
fn create_parent_directory(
    file_system: &dyn FileSystem,
    path: &Path,
) -> Result<(), RepositoryError> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };

    file_system
        .create_dir_all(parent)
        .map_err(|error| RepositoryError::FileSystem {
            operation: "create_dir_all",
            path: parent.to_path_buf(),
            message: error.to_string(),
        })?;

    Ok(())
}
