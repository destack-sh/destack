use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::str;

use destack_source::{FileId, FileMetadata, FileType, matches};

use crate::{Dependency, DestackFile, Edit, Repository, RepositoryError, Revision};

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

    /// Return whether one physical path belongs to its workspace repository.
    fn includes(&self, path: &Path) -> Result<bool, RepositoryError> {
        let directory = path.parent().unwrap_or(self.root);
        let Some(config) = self.package(directory)? else {
            return Ok(Self::tracks_path(path));
        };
        let package_path = self.package_path(&config.directory, path)?;

        Ok(!config.excludes_source(&package_path)
            && (Self::tracks_path(path) || config.includes_source(&package_path)))
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

    /// Scan repository files for one package root.
    fn scan_package_files(
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

        self.scan_files(package_root, config)
    }

    /// Scan recognized and explicitly selected files under one package root.
    fn scan_files(
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

                // collect recognized and explicitly selected files
                let package_path = self.package_path(package_root, &path)?;
                let is_included = Self::tracks_path(&path) || config.includes_source(&package_path);
                if metadata.is_file && is_included {
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

            self.scan_package_files(&package_root, &config)?;
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
