use std::collections::HashSet;
use std::path::{Path, PathBuf};

use destack_source::{FileContent, FileMetadata, FileType, IgnoreSet};
use destack_workspace::Repository;

use super::{RepositorySource, SourceError};

/// Filesystem-backed repository source.
pub(crate) struct FileSystemSource<'a> {
    /// The repository receiving filesystem truth.
    repository: &'a Repository,
    /// The filesystem root scanned by this source.
    root: &'a Path,
    /// The repository path where this source root is mounted.
    mount_path: PathBuf,
    /// The directories waiting to be scanned.
    pending_directories: Vec<PathBuf>,
    /// The directories already scanned.
    visited_directories: HashSet<PathBuf>,
    /// The loaded ignore rules for the scanned root.
    ignore_set: IgnoreSet,
    /// The directory names excluded from recursive scans.
    excluded_directory_names: &'a [&'a str],
    /// The repository path predicate for full source syncs.
    include_path: fn(&Path) -> bool,
}

/// One filesystem file visible to a repository source scan.
pub(crate) struct FileSystemFile {
    /// The physical filesystem path.
    path: PathBuf,
    /// The logical repository path.
    repository_path: PathBuf,
}

impl<'a> FileSystemSource<'a> {
    /// Create one filesystem repository source.
    pub(crate) fn new(repository: &'a Repository, root: &'a Path) -> Self {
        Self::mounted(repository, root, PathBuf::new())
    }

    /// Create one mounted filesystem repository source.
    pub(crate) fn mounted(
        repository: &'a Repository,
        root: &'a Path,
        mount_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            repository,
            root,
            mount_path: mount_path.into(),
            pending_directories: Vec::new(),
            visited_directories: HashSet::new(),
            ignore_set: IgnoreSet::new(),
            excluded_directory_names: &[],
            include_path: |_| true,
        }
    }

    /// Set directory names excluded from recursive scans.
    pub(crate) fn with_excluded_directory_names(
        mut self,
        excluded_directory_names: &'a [&'a str],
    ) -> Self {
        self.excluded_directory_names = excluded_directory_names;

        self
    }

    /// Set the repository path predicate for full source syncs.
    pub(crate) fn with_include_path(mut self, include_path: fn(&Path) -> bool) -> Self {
        self.include_path = include_path;

        self
    }

    /// Return one file descriptor when the path is visible as a file.
    fn file(&mut self, logical_path: &Path) -> Result<Option<FileSystemFile>, SourceError> {
        let Some(path) = self.physical_path(logical_path) else {
            return Ok(None);
        };

        // source visibility
        self.load_ignore_rules_for_path(&path);
        if !self.owns(logical_path)? {
            return Ok(None);
        }

        // filesystem presence
        let metadata = match self.repository.file_system().metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
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

        // files only
        if !metadata.is_file {
            return Ok(None);
        }

        Ok(Some(FileSystemFile {
            path,
            repository_path: logical_path.to_path_buf(),
        }))
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
    fn path_metadata(&self, path: &Path) -> Result<FileMetadata, SourceError> {
        self.repository
            .file_system()
            .metadata(path)
            .map_err(|error| SourceError::ReadFailed {
                operation: "metadata",
                path: path.to_path_buf(),
                message: error.to_string(),
            })
    }

    /// Return whether this source should descend into one directory.
    fn should_scan_directory(&self, path: &Path) -> bool {
        // anonymous paths are not excluded by name
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            return true;
        };

        !self.excluded_directory_names.contains(&name)
    }

    /// Return whether one path is below an excluded directory.
    fn is_below_excluded_directory(&self, path: &Path) -> bool {
        // paths outside the source are handled by ownership
        let Ok(relative_path) = path.strip_prefix(self.root) else {
            return false;
        };

        // excluded directory components
        relative_path.components().any(|component| {
            let name = component.as_os_str().to_string_lossy();

            self.excluded_directory_names.contains(&name.as_ref())
        })
    }

    /// Load ignore rules that can affect one path.
    fn load_ignore_rules_for_path(&mut self, path: &Path) {
        // ignore files cannot affect a path without a parent
        let Some(parent) = path.parent() else {
            return;
        };

        // collect ancestors inside this source
        let mut directories = Vec::new();
        let mut directory = parent;
        loop {
            // stop outside this source root
            if !directory.starts_with(self.root) {
                break;
            }

            // collect the current ancestor
            directories.push(directory.to_path_buf());

            // stop after including the root
            if directory == self.root {
                break;
            }

            // move toward the root
            let Some(parent) = directory.parent() else {
                break;
            };
            directory = parent;
        }

        // load rules from root down to the path parent
        for directory in directories.into_iter().rev() {
            self.ignore_set.load_dir(&directory);
        }
    }

    /// Resolve one logical repository path into this source's physical path space.
    fn physical_path(&self, logical_path: &Path) -> Option<PathBuf> {
        // absolute paths are never owned by repository sources
        if logical_path.is_absolute() {
            return None;
        }

        // map repository path through the source mount
        let relative_path = self.source_relative_path(logical_path)?;

        Some(self.root.join(relative_path))
    }

    /// Resolve one physical source path into repository path space.
    fn repository_path(&self, path: &Path) -> PathBuf {
        let relative_path = path.strip_prefix(self.root).unwrap_or(path);
        let repository_path = self.mount_path.join(relative_path);
        let repository_path = normalize_source_path(repository_path.to_string_lossy());

        PathBuf::from(repository_path)
    }

    /// Resolve one repository path into source relative path space.
    fn source_relative_path(&self, path: &Path) -> Option<PathBuf> {
        // root mounted sources own ordinary relative paths directly
        if self.mount_path.as_os_str().is_empty() {
            return Some(path.to_path_buf());
        }

        // mounted sources own paths below their mount only
        path.strip_prefix(&self.mount_path).ok().map(PathBuf::from)
    }

    /// Load one source file content.
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

/// Normalize one repository source path.
fn normalize_source_path(path: impl AsRef<str>) -> String {
    path.as_ref().replace('\\', "/")
}

impl RepositorySource for FileSystemSource<'_> {
    type File = FileSystemFile;

    fn list(&mut self) -> Result<Vec<Self::File>, SourceError> {
        let mut files = Vec::new();

        // reset mutable traversal state
        self.pending_directories.clear();
        self.visited_directories.clear();
        self.ignore_set = IgnoreSet::new();

        // seed this traversal
        self.pending_directories.push(self.root.to_path_buf());

        // walk source directories
        while let Some(directory) = self.pending_directories.pop() {
            // skip directories already reached through another path
            if !self.visited_directories.insert(directory.clone()) {
                continue;
            }

            // load local ignore rules before classifying children
            self.ignore_set.load_dir(&directory);

            // scan child paths
            for path in self.read_directory(&directory)? {
                // read child metadata once
                let metadata = self.path_metadata(&path)?;

                // ignore paths excluded by loaded rules
                if self
                    .ignore_set
                    .is_ignored(self.root, path.as_path(), metadata.is_directory)
                {
                    continue;
                }

                // queue source directories
                if metadata.is_directory {
                    // descend into included directories
                    if self.should_scan_directory(&path) {
                        self.pending_directories.push(path);
                    }

                    continue;
                }

                // collect filesystem files
                if metadata.is_file {
                    let repository_path = self.repository_path(&path);

                    // skip files outside this source sync
                    if !(self.include_path)(&repository_path) {
                        continue;
                    }

                    files.push(FileSystemFile {
                        repository_path,
                        path,
                    });
                }
            }
        }

        Ok(files)
    }

    fn get(&mut self, path: &Path) -> Result<Option<Self::File>, SourceError> {
        self.file(path)
    }

    fn path<'file>(&self, file: &'file Self::File) -> &'file Path {
        &file.repository_path
    }

    fn read(&mut self, file: &Self::File) -> Result<FileContent, SourceError> {
        self.read_content(&file.path)
    }

    fn owns(&mut self, path: &Path) -> Result<bool, SourceError> {
        let Some(physical_path) = self.physical_path(path) else {
            return Ok(false);
        };
        self.load_ignore_rules_for_path(&physical_path);

        // repository path authority
        if !(self.include_path)(path) {
            return Ok(false);
        }

        // root authority
        if !physical_path.starts_with(self.root) {
            return Ok(false);
        }

        // excluded directory authority
        if self.is_below_excluded_directory(&physical_path) {
            return Ok(false);
        }

        // ignore authority
        Ok(!self.ignore_set.is_ignored(self.root, &physical_path, false))
    }

    fn exists(&mut self, path: &Path) -> Result<bool, SourceError> {
        let Some(physical_path) = self.physical_path(path) else {
            return Ok(false);
        };

        // source authority
        if !self.owns(path)? {
            return Ok(false);
        }

        // filesystem presence
        let metadata = match self.repository.file_system().metadata(&physical_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(false);
            }
            Err(error) => {
                return Err(SourceError::ReadFailed {
                    operation: "metadata",
                    path: physical_path,
                    message: error.to_string(),
                });
            }
        };

        Ok(metadata.is_file)
    }
}
