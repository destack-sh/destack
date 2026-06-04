use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::{
    File, FileContent, FileId, FileMetadata, FileSystem, FileType, Uri, matches as glob_matches,
};
use destack_workspace::{Dependency, DestackFile, Repository, RepositoryError};

use super::{Source, SourceError, SourceImport};
use crate::SessionError;

const SOURCE_EXCLUDED_DIRECTORY_NAMES: &[&str] =
    &[".destack", ".git", "node_modules", "target", "vendor"];

/// Filesystem-backed source importer.
pub(crate) struct FileSystemSource<'a> {
    /// The repository receiving filesystem truth.
    repository: &'a Repository,
    /// The physical source root.
    root: &'a Path,
    /// The source import built so far.
    source_import: SourceImport,
    /// Package roots still to expand.
    packages: Vec<(PathBuf, DestackFile)>,
    /// Package roots already scheduled.
    seen_packages: HashSet<PathBuf>,
}

impl<'a> FileSystemSource<'a> {
    /// Create one filesystem source.
    pub(crate) fn new(repository: &'a Repository, root: &'a Path) -> Self {
        Self {
            repository,
            root,
            source_import: SourceImport::complete(),
            packages: Vec::new(),
            seen_packages: HashSet::new(),
        }
    }

    /// Read one source file by logical repository path.
    pub(crate) fn file(&self, logical_path: &Path) -> Result<Option<SourceImport>, SourceError> {
        let Some(path) = self.physical_path(logical_path) else {
            return Ok(None);
        };

        // read metadata before loading content
        let metadata = match self.repository.file_system().metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(error) => {
                return Err(SourceError::ReadFailed {
                    operation: "metadata",
                    path,
                    message: error.to_string(),
                });
            }
        };
        if !metadata.is_file {
            return Ok(None);
        }

        // build one file import
        let logical_path = self.source_path_text(logical_path);
        let file_id = FileId::from_logical_str(&logical_path);
        let logical_path = self.repository.string_pool().intern(&logical_path);
        let content = self.read_content(&path)?;

        Ok(Some(SourceImport::from_file(
            file_id,
            logical_path,
            content,
        )))
    }

    /// Read one `destack.json` from a physical directory when present.
    fn read_destack_config(&self, root: &Path) -> Result<Option<DestackFile>, RepositoryError> {
        let file_system = self.repository.file_system();

        read_destack_config(file_system.as_ref(), root)
    }

    /// Seed package roots selected by one workspace manifest.
    fn seed_workspace_packages(&mut self, config: DestackFile) -> Result<(), SessionError> {
        let workspace_packages = config.workspace_packages().map(<[_]>::to_vec);

        // add the workspace manifest first
        self.add_config_file(&config)?;

        // seed one package at the source root
        if workspace_packages.is_none() {
            self.add_package(self.root.to_path_buf(), config);
        }
        // seed declared workspace packages
        else if let Some(patterns) = workspace_packages.as_deref() {
            for config_path in self.discover_destack_config_paths()? {
                let package_root = config_path.parent().unwrap_or(self.root).to_path_buf();
                if !self.matches_workspace_package(&package_root, patterns) {
                    continue;
                }

                let Some(config) = self.read_destack_config(&package_root)? else {
                    continue;
                };
                self.add_package(package_root, config);
            }
        }

        Ok(())
    }

    /// Add one package root when it has not already been queued.
    fn add_package(&mut self, package_root: PathBuf, config: DestackFile) {
        if self.seen_packages.insert(package_root.clone()) {
            self.packages.push((package_root, config));
        }
    }

    /// Discover `destack.json` files below the source root.
    fn discover_destack_config_paths(&self) -> Result<Vec<PathBuf>, SourceError> {
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
                if metadata.is_file
                    && path.file_name().and_then(|name| name.to_str()) == Some("destack.json")
                {
                    configs.push(path);
                }
                // descend into source directories
                else if metadata.is_directory && self.should_scan_directory(&path) {
                    pending.push(path);
                }
            }
        }

        configs.sort();
        configs.dedup();

        Ok(configs)
    }

    /// Collect source files for one package root.
    fn collect_package_sources(
        &mut self,
        package_root: &Path,
        config: &DestackFile,
    ) -> Result<(), SourceError> {
        let patterns = config.source_patterns();

        // add the package manifest
        self.add_config_file(config)?;

        // add listed files
        for file in patterns.files {
            let path = package_root.join(self.normalize_pattern(&file));
            if self.is_excluded(package_root, &path, &patterns.exclude) {
                continue;
            }
            let metadata = self.metadata(&path)?;
            if metadata.is_file {
                self.add_file(&path)?;
            }
        }

        self.collect_include_paths(package_root, &patterns.include, &patterns.exclude)
    }

    /// Collect included source files under one package root.
    fn collect_include_paths(
        &mut self,
        package_root: &Path,
        include: &[String],
        exclude: &[String],
    ) -> Result<(), SourceError> {
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
                    if self.should_scan_directory(&path)
                        && !self.is_excluded(package_root, &path, exclude)
                    {
                        pending.push(path);
                    }
                    continue;
                }

                // collect matching files
                if metadata.is_file
                    && self.is_included(package_root, &path, include)
                    && !self.is_excluded(package_root, &path, exclude)
                {
                    self.add_file(&path)?;
                }
            }
        }

        Ok(())
    }

    /// Add local path dependency roots declared by one package config.
    fn add_path_dependencies(
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
            let Some(config) = self.read_destack_config(&dependency_root)? else {
                continue;
            };

            self.add_package(dependency_root, config);
        }

        Ok(())
    }

    /// Add one parsed config file to the snapshot.
    fn add_config_file(&mut self, config: &DestackFile) -> Result<(), SourceError> {
        self.add_file(&config.path)
    }

    /// Add one physical file path to the snapshot.
    fn add_file(&mut self, path: &Path) -> Result<(), SourceError> {
        let logical_path = self.logical_path(path);
        let file_id = FileId::from_logical_str(&logical_path);
        let logical_path = self.repository.string_pool().intern(&logical_path);
        let content = self.read_content(path)?;

        self.source_import.add_file(file_id, logical_path, content);

        Ok(())
    }

    /// Return the logical repository path for one physical path.
    fn logical_path(&self, path: &Path) -> String {
        self.repository.logical_path(path)
    }

    /// Return the physical path for one logical repository path.
    fn physical_path(&self, logical_path: &Path) -> Option<PathBuf> {
        if logical_path.is_absolute() {
            None
        } else {
            Some(self.root.join(logical_path))
        }
    }

    /// Return true when one package root is selected by workspace patterns.
    fn matches_workspace_package(&self, package_root: &Path, patterns: &[String]) -> bool {
        patterns
            .iter()
            .any(|pattern| self.matches_workspace_pattern(package_root, pattern))
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

    /// Return true when one physical path matches include patterns.
    fn is_included(&self, package_root: &Path, path: &Path, include: &[String]) -> bool {
        let package_path = self.package_path(package_root, path);

        include
            .iter()
            .any(|pattern| self.pattern_matches(pattern, &package_path))
    }

    /// Return true when one physical path matches exclude patterns.
    fn is_excluded(&self, package_root: &Path, path: &Path, exclude: &[String]) -> bool {
        let package_path = self.package_path(package_root, path);

        exclude
            .iter()
            .any(|pattern| self.pattern_matches(pattern, &package_path))
    }

    /// Return one package-relative path as normalized text.
    fn package_path(&self, package_root: &Path, path: &Path) -> String {
        let package_path = path.strip_prefix(package_root).unwrap_or(path);

        self.source_path_text(package_path)
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

        glob_matches(pattern.as_bytes(), 0, package_path.as_bytes(), 0)
            || package_path == pattern
            || package_path
                .strip_prefix(pattern.as_str())
                .is_some_and(|suffix| suffix.starts_with('/'))
    }

    /// Return whether this source scanner should descend into one directory.
    fn should_scan_directory(&self, path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return true;
        };

        !SOURCE_EXCLUDED_DIRECTORY_NAMES.contains(&name)
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
        self.repository
            .file_system()
            .read_dir(directory)
            .map_err(|error| SourceError::ReadFailed {
                operation: "read_dir",
                path: directory.to_path_buf(),
                message: error.to_string(),
            })
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

impl Source for FileSystemSource<'_> {
    fn import(&mut self) -> Result<SourceImport, SessionError> {
        let Some(workspace_config) = self.read_destack_config(self.root)? else {
            return Ok(std::mem::replace(
                &mut self.source_import,
                SourceImport::complete(),
            ));
        };

        // seed packages from the workspace manifest
        self.seed_workspace_packages(workspace_config)?;

        // expand package sources and local dependencies
        let mut index = 0;
        while index < self.packages.len() {
            let (package_root, config) = self.packages[index].clone();
            index += 1;

            self.collect_package_sources(&package_root, &config)?;
            self.add_path_dependencies(&package_root, &config)?;
        }

        Ok(std::mem::replace(
            &mut self.source_import,
            SourceImport::complete(),
        ))
    }
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

    DestackFile::parse(&file)
        .map(Some)
        .map_err(|error| RepositoryError::WorkspaceRootDiscovery {
            path,
            message: error.to_string(),
        })
}
