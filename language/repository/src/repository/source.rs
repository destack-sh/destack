use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_artifact::{BlobStore, MemoryBlobStore};
use destack_source::{
    Content, ContentId, File, FileId, FileMetadata, FilePatch, FileSystem, FileType,
    MemoryFileSystem, Patch, Span, TextPatch, Uri, apply_file_patch, matches as glob_matches,
};

use crate::{
    Dependency, DestackFile, DestackLayout, DestackLayoutOverride, Edit, Environment, Host, Ref,
    Repository, RepositoryError, Revision, Settings, default_blob_store,
};

/// Open one repository after discovering the source root from one path.
pub fn open_repository_from_fs(
    path: PathBuf,
    file_system: Arc<dyn FileSystem>,
    environment: Environment,
    settings: Settings,
    layout_override: DestackLayoutOverride,
) -> Result<Repository, RepositoryError> {
    let blob_store = default_blob_store();
    let host = Host::new(environment, file_system, blob_store);

    open_repository(path, host, settings, layout_override)
}

/// Open one repository from one in-memory source.
pub fn open_repository_from_memory(
    root: PathBuf,
    edits: Vec<destack_source::Edit>,
    environment: Environment,
    settings: Settings,
    layout_override: DestackLayoutOverride,
) -> Result<Repository, RepositoryError> {
    let file_system = Arc::new(MemoryFileSystem::new());
    apply_source_edits(file_system.as_ref(), &root, edits)?;

    // keep memory repositories fully in memory
    let blob_store: Arc<dyn BlobStore> = Arc::new(MemoryBlobStore::new());
    let host = Host::new(environment, file_system, blob_store);

    open_repository(root, host, settings, layout_override)
}

/// Open one repository from explicit host capabilities.
pub fn open_repository(
    path: PathBuf,
    host: Host,
    settings: Settings,
    layout_override: DestackLayoutOverride,
) -> Result<Repository, RepositoryError> {
    let root = find_source_root(host.files().as_ref(), &path)?;
    let environment = host.environment();
    let cwd = environment.cwd.as_deref().unwrap_or(&path);
    let layout = DestackLayout::resolve(&root, cwd, environment, &settings, &layout_override, None);

    // create repository at the selected source root
    let repository = Repository::new(root.clone(), host, settings, layout);
    let root_ref = Ref::for_root(&root);
    let base_revision = repository.current(&root_ref)?;

    // read the complete source tree
    let source = FileSystemSource::new(&repository, &root, base_revision);
    let edits = source.edits()?;
    let revision = repository.commit_edits(base_revision, edits)?;

    repository.set_ref(&root_ref, revision)?;

    Ok(repository)
}

/// Find the source root for one filesystem input path.
fn find_source_root(file_system: &dyn FileSystem, path: &Path) -> Result<PathBuf, RepositoryError> {
    let metadata =
        file_system
            .metadata(path)
            .map_err(|error| RepositoryError::WorkspaceRootDiscovery {
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;

    // normalize file inputs to their containing directory
    let input_directory = if metadata.is_file {
        path.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| path.to_path_buf())
    } else {
        path.to_path_buf()
    };
    let mut current = input_directory.clone();

    // walk up directories looking for a source root
    loop {
        if FileSystemSource::read_destack_config(file_system, &current)?.is_some() {
            return Ok(current);
        }

        let Some(parent) = current.parent() else {
            break;
        };
        current = parent.to_path_buf();
    }

    Ok(input_directory)
}

/// Filesystem-backed source.
#[derive(Debug)]
pub struct FileSystemSource<'a> {
    /// The repository receiving filesystem truth.
    repository: &'a Repository,
    /// The base repository revision.
    base: Revision,
    /// The physical source root.
    root: &'a Path,
    /// Repository edits built by this source.
    edits: Vec<Edit>,
    /// File ids seen during this source scan.
    seen_file_ids: HashSet<FileId>,
    /// Package roots still to scan.
    pending_packages: Vec<(PathBuf, DestackFile)>,
    /// Package roots already queued.
    queued_package_roots: HashSet<PathBuf>,
}

impl<'a> FileSystemSource<'a> {
    /// Create one filesystem source.
    pub fn new(repository: &'a Repository, root: &'a Path, base: Revision) -> Self {
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

    /// Return whether filesystem source imports one path.
    pub fn tracks_path(path: &Path) -> bool {
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

    /// Return one filesystem path as a source edit.
    pub fn read_edit(
        repository: &Repository,
        path: &Path,
    ) -> std::io::Result<destack_source::Edit> {
        // preserve bytes for binary formats
        if FileType::from_path(path).is_some_and(|file_type| file_type.is_binary()) {
            let content = repository.file_system().read(path)?;
            let path = path.to_path_buf();

            return Ok(destack_source::Edit::SetBytes {
                path,
                bytes: content,
            });
        }

        // use text for source readable files
        let content = repository.file_system().read_to_string(path)?;
        let path = path.to_path_buf();

        Ok(destack_source::Edit::SetText {
            path,
            text: content,
        })
    }

    /// Return whether one path is a destack manifest.
    fn is_destack_config_path(path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == "destack.json")
    }

    /// Read one `destack.json` from a physical directory when present.
    pub fn read_destack_config(
        file_system: &dyn FileSystem,
        root: &Path,
    ) -> Result<Option<DestackFile>, RepositoryError> {
        let path = root.join("destack.json");

        // read config when present
        let content = match file_system.read_to_string(&path) {
            Ok(content) => content,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(RepositoryError::WorkspaceRootDiscovery {
                    path,
                    message: error.to_string(),
                });
            }
        };

        // parse through normal workspace config logic
        let file = File::from_text(
            FileId::from_logical_str("destack.json"),
            "destack.json".to_string(),
            Uri::from_path(&path),
            Some(path.clone()),
            FileType::Json,
            content,
        );
        let file = Arc::new(file);

        DestackFile::parse(&file).map(Some).map_err(|error| {
            RepositoryError::WorkspaceRootDiscovery {
                path,
                message: error.to_string(),
            }
        })
    }

    /// Return repository edits for one logical source file when it exists.
    pub fn repository_edits_for_path(
        mut self,
        logical_path: &Path,
    ) -> Result<Option<Vec<Edit>>, RepositoryError> {
        if logical_path.is_absolute() {
            return Ok(None);
        }
        let path = self.root.join(logical_path);

        // read metadata before loading content
        let metadata = match self.repository.file_system().metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(error) => {
                let error = RepositoryError::FileSystem {
                    operation: "metadata",
                    path,
                    message: error.to_string(),
                };

                return Err(error);
            }
        };
        if !metadata.is_file {
            return Ok(None);
        }

        // add the requested file
        self.import_file_with_logical_path(&path, logical_path)?;

        // relative imports resolve against the file's package: scan the
        // nearest manifest's package, or the file's own directory when
        // no manifest exists
        let directory = path.parent().map(Path::to_path_buf);
        if let Some(directory) = directory {
            match self.find_enclosing_package(&directory)? {
                Some((package_root, config)) => {
                    self.queue_package(package_root, config);
                    self.scan_queued_packages()?;
                }
                None => self.import_sibling_files(&directory, &path)?,
            }
        }

        Ok(Some(self.edits))
    }

    /// Return the nearest enclosing package manifest within the root.
    fn find_enclosing_package(
        &self,
        directory: &Path,
    ) -> Result<Option<(PathBuf, DestackFile)>, RepositoryError> {
        let mut current = Some(directory);
        while let Some(candidate) = current {
            if let Some(config) = self.read_source_config(candidate)? {
                return Ok(Some((candidate.to_path_buf(), config)));
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

    /// Import the trackable files beside one directly loaded file.
    fn import_sibling_files(
        &mut self,
        directory: &Path,
        loaded: &Path,
    ) -> Result<(), RepositoryError> {
        let entries = match self.repository.file_system().read_dir(directory) {
            Ok(entries) => entries,
            Err(_) => return Ok(()),
        };

        for entry in entries {
            if entry == loaded || !Self::tracks_path(&entry) {
                continue;
            }
            let metadata = match self.repository.file_system().metadata(&entry) {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            if metadata.is_file {
                self.import_file(&entry)?;
            }
        }

        Ok(())
    }

    /// Read one `destack.json` from a physical directory when present.
    fn read_source_config(&self, root: &Path) -> Result<Option<DestackFile>, RepositoryError> {
        let file_system = self.repository.file_system();

        Self::read_destack_config(file_system.as_ref(), root)
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
            let source_patterns = config.source_patterns();
            for config_path in self.find_destack_config_paths(&source_patterns.exclude)? {
                let package_root = config_path.parent().unwrap_or(self.root).to_path_buf();
                let is_workspace_package = patterns
                    .iter()
                    .any(|pattern| self.matches_workspace_pattern(&package_root, pattern));
                if !is_workspace_package {
                    continue;
                }

                let Some(config) = self.read_source_config(&package_root)? else {
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
        exclude: &[String],
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
                    let package_path = self.package_path(self.root, &path);
                    if !self.patterns_match(exclude, &package_path) {
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
        let patterns = config.source_patterns();

        // import the package manifest
        self.import_file(&config.path)?;

        // add listed files
        for file in patterns.files {
            let path = package_root.join(self.normalize_pattern(&file));
            let package_path = self.package_path(package_root, &path);
            if self.patterns_match(&patterns.exclude, &package_path) {
                continue;
            }

            let metadata = self.metadata(&path)?;
            if metadata.is_file {
                self.import_file(&path)?;
            }
        }

        self.scan_include_paths(package_root, &patterns.include, &patterns.exclude)
    }

    /// Scan included source files under one package root.
    fn scan_include_paths(
        &mut self,
        package_root: &Path,
        include: &[String],
        exclude: &[String],
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
                    let package_path = self.package_path(package_root, &path);
                    let is_excluded = self.patterns_match(exclude, &package_path);
                    if !is_excluded {
                        pending.push(path);
                    }
                    continue;
                }

                // collect matching files
                let package_path = self.package_path(package_root, &path);
                let is_included = self.patterns_match(include, &package_path);
                let is_excluded = self.patterns_match(exclude, &package_path);
                if metadata.is_file && is_included && !is_excluded {
                    self.import_file(&path)?;
                }
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
            let dependency_root = self.normalize_path(package_root.join(path));
            let Some(config) = self.read_source_config(&dependency_root)? else {
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

    /// Import one physical file path into the source scan.
    fn import_file(&mut self, path: &Path) -> Result<(), RepositoryError> {
        let logical_path = self.repository.logical_path(path);

        self.import_file_with_logical_path(path, Path::new(&logical_path))
    }

    /// Import one physical file with one logical path into the source scan.
    fn import_file_with_logical_path(
        &mut self,
        path: &Path,
        logical_path: &Path,
    ) -> Result<(), RepositoryError> {
        let logical_path = self.source_path_text(logical_path);
        let file_id = FileId::from_logical_str(&logical_path);
        let content = self.read_content(path)?;

        self.seen_file_ids.insert(file_id);
        let incoming = ContentId::for_content(&content);
        let current = self.repository.file_content_id(self.base, file_id)?;

        // record changed files only
        if current != Some(incoming) {
            self.edits.push(Edit::SetFile {
                logical_path,
                content,
            });
        }

        Ok(())
    }

    /// Return true when one workspace package pattern matches one root.
    fn matches_workspace_pattern(&self, package_root: &Path, pattern: &str) -> bool {
        let package_path = package_root
            .strip_prefix(self.root)
            .unwrap_or(package_root)
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

        glob_matches(pattern.as_bytes(), 0, package_path.as_bytes(), 0)
            || glob_matches(pattern.as_bytes(), 0, config_path.as_bytes(), 0)
    }

    /// Return one package-relative path as normalized text.
    fn package_path(&self, package_root: &Path, path: &Path) -> String {
        let package_path = path.strip_prefix(package_root).unwrap_or(path);

        self.source_path_text(package_path)
    }

    /// Return whether any source pattern matches one package-relative path.
    fn patterns_match(&self, patterns: &[String], package_path: &str) -> bool {
        patterns
            .iter()
            .any(|pattern| self.pattern_matches(pattern, package_path))
    }

    /// Normalize one package-relative source pattern.
    fn normalize_pattern(&self, pattern: &str) -> String {
        pattern
            .replace('\\', "/")
            .trim_start_matches("./")
            .to_string()
    }

    /// Return whether one source pattern matches one package-relative path.
    fn pattern_matches(&self, pattern: &str, package_path: &str) -> bool {
        let pattern = self.normalize_pattern(pattern);
        let directory_pattern = pattern.strip_suffix("/**");

        glob_matches(pattern.as_bytes(), 0, package_path.as_bytes(), 0)
            || package_path == pattern
            || directory_pattern.is_some_and(|pattern| package_path == pattern)
            || package_path
                .strip_prefix(pattern.as_str())
                .is_some_and(|suffix| suffix.starts_with('/'))
    }

    /// Return one path as normalized source text.
    fn source_path_text(&self, path: impl AsRef<Path>) -> String {
        path.as_ref().to_string_lossy().replace('\\', "/")
    }

    /// Normalize one lexical path.
    fn normalize_path(&self, path: PathBuf) -> PathBuf {
        let mut normalized = PathBuf::new();

        // rebuild without current or parent components
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

    /// Read one source file content.
    fn read_content(&self, path: &Path) -> Result<Content, RepositoryError> {
        let file_type = FileType::from_path_or_unknown(path);

        // binary file content
        if file_type.is_binary() {
            let content = self.repository.file_system().read(path).map_err(|error| {
                RepositoryError::FileSystem {
                    operation: "read",
                    path: path.to_path_buf(),
                    message: error.to_string(),
                }
            })?;

            Ok(Content::Binary { content })
        }
        // text file content
        else {
            let content = self
                .repository
                .file_system()
                .read_to_string(path)
                .map_err(|error| RepositoryError::FileSystem {
                    operation: "read_to_string",
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })?;

            Ok(Content::Text { content })
        }
    }
}

impl FileSystemSource<'_> {
    /// Return the repository edits for the complete source state.
    pub fn edits(mut self) -> Result<Vec<Edit>, RepositoryError> {
        self.scan()?;
        self.remove_missing_files()?;

        Ok(self.edits)
    }

    /// Scan the complete source state visible from this filesystem source.
    fn scan(&mut self) -> Result<(), RepositoryError> {
        let Some(workspace_config) = self.read_source_config(self.root)? else {
            return Ok(());
        };

        // queue packages from the workspace manifest
        self.queue_workspace_packages(workspace_config)?;

        self.scan_queued_packages()
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
}

/// Apply source edits to one filesystem root.
fn apply_source_edits(
    file_system: &dyn FileSystem,
    root: &Path,
    edits: Vec<destack_source::Edit>,
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
        apply_source_edit(file_system, root, edit)?;
    }

    Ok(())
}

/// Apply one source edit to one filesystem root.
fn apply_source_edit(
    file_system: &dyn FileSystem,
    root: &Path,
    edit: destack_source::Edit,
) -> Result<(), RepositoryError> {
    match edit {
        destack_source::Edit::SetText { path, text } => {
            let path = root.join(path);

            write_text(file_system, &path, &text)
        }
        destack_source::Edit::SetBytes { path, bytes } => {
            let path = root.join(path);

            write_bytes(file_system, &path, &bytes)
        }
        destack_source::Edit::EditText { path, patches } => {
            let full_path = root.join(&path);

            patch_text(file_system, &full_path, &path, patches)
        }
        destack_source::Edit::Remove { path } => {
            let path = root.join(path);

            remove_path(file_system, &path)
        }
        destack_source::Edit::Move { from, to } => {
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
    let file_id = FileId::from_logical_path(logical_path);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();
    let file = File::from_text(
        file_id,
        name,
        Uri::from_file_path(path),
        Some(path.to_path_buf()),
        FileType::from_path_or_unknown(path),
        text,
    );

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
