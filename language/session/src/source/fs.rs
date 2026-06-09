use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_repository::{Dependency, DestackFile, Repository, RepositoryError, Revision};
use destack_source::{
    File, FileContent, FileContentId, FileId, FileMetadata, FileSystem, FileType, Uri,
    matches as glob_matches,
};

use super::SourceError;
use crate::{Edit, SessionError};

/// Filesystem-backed source.
pub(crate) struct FileSystemSource<'a> {
    /// The repository receiving filesystem truth.
    repository: &'a Repository,
    /// The base repository revision.
    base: Revision,
    /// The physical source root.
    root: &'a Path,
    /// Repository edits built by this source.
    edits: Vec<destack_repository::Edit>,
    /// File ids seen during this source scan.
    seen_file_ids: HashSet<FileId>,
    /// Package roots still to scan.
    pending_packages: Vec<(PathBuf, DestackFile)>,
    /// Package roots already queued.
    queued_package_roots: HashSet<PathBuf>,
}

impl<'a> FileSystemSource<'a> {
    /// Create one filesystem source.
    pub(crate) fn new(repository: &'a Repository, root: &'a Path, base: Revision) -> Self {
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
    pub(crate) fn tracks_path(path: &Path) -> bool {
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

    /// Return one filesystem path as a session edit.
    pub(crate) fn read_edit(repository: &Repository, path: &Path) -> std::io::Result<Edit> {
        // preserve bytes for binary formats
        if FileType::from_path(path).is_some_and(|file_type| file_type.is_binary()) {
            let content = repository.file_system().read(path)?;
            let path = path.to_path_buf();

            return Ok(Edit::SetBytes {
                path,
                bytes: content,
            });
        }

        // use text for source readable files
        let content = repository.file_system().read_to_string(path)?;
        let path = path.to_path_buf();

        Ok(Edit::SetText {
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
    pub(crate) fn read_destack_config(
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
    pub(crate) fn repository_edits_for_path(
        mut self,
        logical_path: &Path,
    ) -> Result<Option<Vec<destack_repository::Edit>>, SessionError> {
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
                let error = SourceError::ReadFailed {
                    operation: "metadata",
                    path,
                    message: error.to_string(),
                };

                return Err(SessionError::from(error));
            }
        };
        if !metadata.is_file {
            return Ok(None);
        }

        // add the requested file
        self.import_file_with_logical_path(&path, logical_path)?;

        Ok(Some(self.edits))
    }

    /// Read one `destack.json` from a physical directory when present.
    fn read_source_config(&self, root: &Path) -> Result<Option<DestackFile>, RepositoryError> {
        let file_system = self.repository.file_system();

        Self::read_destack_config(file_system.as_ref(), root)
    }

    /// Queue package roots selected by one workspace manifest.
    fn queue_workspace_packages(&mut self, config: DestackFile) -> Result<(), SessionError> {
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
    fn find_destack_config_paths(&self, exclude: &[String]) -> Result<Vec<PathBuf>, SourceError> {
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
    ) -> Result<(), SessionError> {
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
    ) -> Result<(), SessionError> {
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
    ) -> Result<(), SessionError> {
        let dependencies = config.dependencies.values().chain(
            config
                .conditional_dependencies
                .iter()
                .flat_map(|conditional| conditional.dependencies.values()),
        );

        // add filesystem path dependencies
        for dependency in dependencies {
            let Dependency::Path { path } = dependency else {
                continue;
            };
            let dependency_root = self.normalize_path(package_root.join(path));
            let Some(config) = self.read_source_config(&dependency_root)? else {
                continue;
            };

            self.queue_package(dependency_root, config);
        }

        Ok(())
    }

    /// Import one physical file path into the source scan.
    fn import_file(&mut self, path: &Path) -> Result<(), SessionError> {
        let logical_path = self.repository.logical_path(path);

        self.import_file_with_logical_path(path, Path::new(&logical_path))
    }

    /// Import one physical file with one logical path into the source scan.
    fn import_file_with_logical_path(
        &mut self,
        path: &Path,
        logical_path: &Path,
    ) -> Result<(), SessionError> {
        let logical_path = self.source_path_text(logical_path);
        let file_id = FileId::from_logical_str(&logical_path);
        let content = self.read_content(path)?;

        self.seen_file_ids.insert(file_id);
        let incoming = FileContentId::for_content(&content);
        let current = self.repository.file_content_id(self.base, file_id)?;

        // record changed files only
        if current != Some(incoming) {
            self.edits.push(destack_repository::Edit::SetFile {
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
    fn read_directory(&self, directory: &Path) -> Result<Vec<PathBuf>, SourceError> {
        let mut paths = self
            .repository
            .file_system()
            .read_dir(directory)
            .map_err(|error| SourceError::ReadFailed {
                operation: "read_dir",
                path: directory.to_path_buf(),
                message: error.to_string(),
            })?;
        paths.sort();

        Ok(paths)
    }

    /// Read metadata for one path.
    fn metadata(&self, path: &Path) -> Result<FileMetadata, SourceError> {
        self.repository
            .file_system()
            .metadata(path)
            .map_err(|error| SourceError::ReadFailed {
                operation: "metadata",
                path: path.to_path_buf(),
                message: error.to_string(),
            })
    }

    /// Read one source file content.
    fn read_content(&self, path: &Path) -> Result<FileContent, SourceError> {
        let file_type = FileType::from_path_or_unknown(path);

        // binary file content
        if file_type.is_binary() {
            let content = self.repository.file_system().read(path).map_err(|error| {
                SourceError::ReadFailed {
                    operation: "read",
                    path: path.to_path_buf(),
                    message: error.to_string(),
                }
            })?;

            Ok(FileContent::Binary { content })
        }
        // text file content
        else {
            let content = self
                .repository
                .file_system()
                .read_to_string(path)
                .map_err(|error| SourceError::ReadFailed {
                    operation: "read_to_string",
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })?;

            Ok(FileContent::Text { content })
        }
    }
}

impl FileSystemSource<'_> {
    /// Return the repository edits for the complete source state.
    pub(crate) fn edits(mut self) -> Result<Vec<destack_repository::Edit>, SessionError> {
        self.scan()?;
        self.remove_missing_files()?;

        Ok(self.edits)
    }

    /// Scan the complete source state visible from this filesystem source.
    fn scan(&mut self) -> Result<(), SessionError> {
        let Some(workspace_config) = self.read_source_config(self.root)? else {
            return Ok(());
        };

        // queue packages from the workspace manifest
        self.queue_workspace_packages(workspace_config)?;

        // expand package sources and local dependencies
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
    fn remove_missing_files(&mut self) -> Result<(), SessionError> {
        for (file_id, logical_path) in self.repository.editable_file_logical_paths(self.base)? {
            if !self.seen_file_ids.contains(&file_id) {
                let logical_path = self.repository.string_pool().get(logical_path);
                self.edits
                    .push(destack_repository::Edit::remove_file(logical_path));
            }
        }

        Ok(())
    }
}
