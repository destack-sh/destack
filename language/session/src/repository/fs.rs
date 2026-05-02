use std::collections::HashSet;
use std::path::{Path, PathBuf};

use destack_source::{FileContent, FileMetadata, IgnoreSet};
use destack_workspace::{Repository, RepositoryError};

use super::RepositorySource;

/// Filesystem-backed repository source.
pub(crate) struct FileSystemSource<'a> {
    /// The repository receiving filesystem truth.
    repository: &'a Repository,
    /// The filesystem root scanned by this source.
    root: &'a Path,
    /// The directories waiting to be scanned.
    pending_directories: Vec<PathBuf>,
    /// The directories already scanned.
    visited_directories: HashSet<PathBuf>,
    /// The loaded ignore rules for the scanned root.
    ignore_set: IgnoreSet,
    /// The directory names excluded from recursive scans.
    excluded_directory_names: &'a [&'a str],
    /// The repository path predicate for full source syncs.
    tracked_path: fn(&Path) -> bool,
}

/// One filesystem file visible to a repository source scan.
pub(crate) struct FileSystemFile {
    /// The physical filesystem path.
    path: PathBuf,
    /// The repository path.
    repository_path: PathBuf,
}

impl<'a> FileSystemSource<'a> {
    /// Create one filesystem repository source.
    pub(crate) fn new(repository: &'a Repository, root: &'a Path) -> Self {
        Self {
            repository,
            root,
            pending_directories: Vec::new(),
            visited_directories: HashSet::new(),
            ignore_set: IgnoreSet::new(),
            excluded_directory_names: &[],
            tracked_path: |_| true,
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
    pub(crate) fn with_tracked_path(mut self, tracked_path: fn(&Path) -> bool) -> Self {
        self.tracked_path = tracked_path;

        self
    }

    /// Return one file descriptor when the path is visible as a file.
    fn file(&mut self, path: &Path) -> Result<Option<FileSystemFile>, RepositoryError> {
        // source visibility
        self.load_ignore_rules_for_path(path);
        if !self.tracks(path) {
            return Ok(None);
        }

        // filesystem presence
        let metadata = match self.repository.file_system().metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None);
            }
            Err(error) => {
                return Err(RepositoryError::FileSystem {
                    operation: "metadata",
                    path: path.to_path_buf(),
                    message: error.to_string(),
                });
            }
        };

        // files only
        if !metadata.is_file {
            return Ok(None);
        }

        Ok(Some(FileSystemFile {
            path: path.to_path_buf(),
            repository_path: PathBuf::from(self.repository.logical_path(path)),
        }))
    }

    /// Read child paths for one directory.
    fn read_directory(&self, directory: &Path) -> Result<Vec<PathBuf>, RepositoryError> {
        self.repository
            .file_system()
            .read_dir(directory)
            .map_err(|error| RepositoryError::FileSystem {
                operation: "read_dir",
                path: directory.to_path_buf(),
                message: error.to_string(),
            })
    }

    /// Read metadata for one path.
    fn path_metadata(&self, path: &Path) -> Result<FileMetadata, RepositoryError> {
        self.repository
            .file_system()
            .metadata(path)
            .map_err(|error| RepositoryError::FileSystem {
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
        // paths outside the source are handled by tracks
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
}

impl RepositorySource for FileSystemSource<'_> {
    type File = FileSystemFile;

    fn list(&mut self) -> Result<Vec<Self::File>, RepositoryError> {
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
                    let repository_path = PathBuf::from(self.repository.logical_path(&path));

                    // skip files outside this source sync
                    if !(self.tracked_path)(&repository_path) {
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

    fn get(&mut self, path: &Path) -> Result<Option<Self::File>, RepositoryError> {
        self.file(path)
    }

    fn path<'file>(&self, file: &'file Self::File) -> &'file Path {
        &file.repository_path
    }

    fn read(&self, file: &Self::File) -> Result<FileContent, RepositoryError> {
        self.repository.load_workspace_file_content(&file.path)
    }

    fn tracks(&self, path: &Path) -> bool {
        // root authority
        if !path.starts_with(self.root) {
            return false;
        }

        // excluded directory authority
        if self.is_below_excluded_directory(path) {
            return false;
        }

        // ignore authority
        if self.ignore_set.is_ignored(self.root, path, false) {
            return false;
        }

        // repository path authority
        let repository_path = self.repository.logical_path(path);

        (self.tracked_path)(Path::new(&repository_path))
    }

    fn has(&self, path: &Path) -> Result<bool, RepositoryError> {
        // source authority
        if !self.tracks(path) {
            return Ok(false);
        }

        // filesystem presence
        let metadata = match self.repository.file_system().metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(false);
            }
            Err(error) => {
                return Err(RepositoryError::FileSystem {
                    operation: "metadata",
                    path: path.to_path_buf(),
                    message: error.to_string(),
                });
            }
        };

        Ok(metadata.is_file)
    }
}
