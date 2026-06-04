use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use destack_source::{FileContent, FileContentId, FileId, FileSystem, StringId};
use destack_workspace::{Edit, Repository, RepositoryChange, Revision};

use crate::{SessionError, SourceError};

/// Source that can import one source root into repository-ready state.
pub(crate) trait Source {
    /// Import the complete source state visible from this source.
    fn import(&mut self) -> Result<SourceImport, SessionError>;
}

/// Source truth used to open a live session without native filesystem access.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SourceSnapshot {
    /// Files visible to the session source root.
    pub files: Vec<SourceFile>,
}

/// One file in a source snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    /// Repository-root relative path.
    pub path: PathBuf,
    /// Full source file content.
    pub content: SourceFileContent,
}

/// Full content for one source snapshot file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceFileContent {
    /// Text file content.
    Text(String),
    /// Binary file content.
    Bytes(Vec<u8>),
}

/// Source root state ready to import into a repository.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SourceImport {
    /// Files visible in this import by stable file id.
    files: BTreeMap<FileId, (StringId, FileContent)>,
    /// Whether missing editable files should be removed.
    remove_missing_files: bool,
}

impl SourceSnapshot {
    /// Create one source snapshot from explicit files.
    pub fn new(files: Vec<SourceFile>) -> Self {
        Self { files }
    }

    /// Write this source snapshot into one filesystem root.
    pub(crate) fn write_to(
        self,
        file_system: &dyn FileSystem,
        root: &Path,
    ) -> Result<(), SessionError> {
        file_system
            .create_dir_all(root)
            .map_err(|error| SourceError::WriteFailed {
                operation: "create_dir_all",
                path: root.to_path_buf(),
                message: error.to_string(),
            })?;

        // write every snapshot file under the source root
        for file in self.files {
            let path = root.join(file.path);
            match file.content {
                SourceFileContent::Text(text) => {
                    file_system.write_string(&path, &text).map_err(|error| {
                        SourceError::WriteFailed {
                            operation: "write_string",
                            path: path.clone(),
                            message: error.to_string(),
                        }
                    })?;
                }
                SourceFileContent::Bytes(bytes) => {
                    file_system
                        .write(&path, &bytes)
                        .map_err(|error| SourceError::WriteFailed {
                            operation: "write",
                            path: path.clone(),
                            message: error.to_string(),
                        })?;
                }
            }
        }

        Ok(())
    }
}

impl SourceFile {
    /// Create one text source file.
    pub fn text(path: impl Into<PathBuf>, text: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: SourceFileContent::Text(text.into()),
        }
    }

    /// Create one binary source file.
    pub fn bytes(path: impl Into<PathBuf>, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            path: path.into(),
            content: SourceFileContent::Bytes(bytes.into()),
        }
    }
}

impl SourceImport {
    /// Create one complete source import.
    pub(crate) fn complete() -> Self {
        Self {
            files: BTreeMap::new(),
            remove_missing_files: true,
        }
    }

    /// Create one source import containing one explicit file.
    pub(crate) fn from_file(file_id: FileId, logical_path: StringId, content: FileContent) -> Self {
        let mut import = Self::default();
        import.add_file(file_id, logical_path, content);

        import
    }

    /// Add one source import file.
    pub(crate) fn add_file(
        &mut self,
        file_id: FileId,
        logical_path: StringId,
        content: FileContent,
    ) {
        self.files.insert(file_id, (logical_path, content));
    }

    /// Return the repository change needed to sync this import.
    pub(crate) fn change(
        &self,
        repository: &Repository,
        revision: Revision,
    ) -> Result<RepositoryChange, SessionError> {
        let mut change = RepositoryChange::new();

        // add changed import files
        for (file_id, (logical_path, content)) in &self.files {
            let incoming = FileContentId::for_content(content);
            let current = repository.file_content_id(revision, *file_id)?;

            if current != Some(incoming) {
                let logical_path = repository.string_pool().get(*logical_path).to_string();
                change.push(Edit::SetFile {
                    logical_path,
                    content: content.clone(),
                });
            }
        }

        // remove missing files for full source imports
        if self.remove_missing_files {
            for (file_id, logical_path) in repository.editable_file_logical_paths(revision)? {
                if !self.files.contains_key(&file_id) {
                    let logical_path = repository.string_pool().get(logical_path);
                    change.push(Edit::remove_file(logical_path));
                }
            }
        }

        Ok(change)
    }
}
