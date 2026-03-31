use std::sync::Arc;

use destack_source::FileId;
use destack_workspace::workspace::ModuleSource;
use std::path::{Path, PathBuf};

use crate::repository::Repository;

/// Normalize one logical file path string.
pub(crate) fn normalize_logical_path_str(value: &str) -> String {
    value.replace('\\', "/")
}

/// Normalize one logical file path.
pub(crate) fn normalize_logical_path(path: &Path) -> String {
    normalize_logical_path_str(&path.to_string_lossy())
}

/// The origin metadata for one file identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileOrigin {
    /// One workspace rooted file.
    Workspace { path: PathBuf },
    /// One builtin file.
    Builtin {
        /// The module path under the builtin package.
        module_path: String,
        /// The builtin source kind.
        source: ModuleSource,
    },
    /// The synthetic root file.
    Root,
}

impl Repository {
    /// Normalize one logical path for one workspace file path.
    pub fn normalize_workspace_path(&self, path: &Path) -> String {
        let logical_path = path.strip_prefix(&self.workspace.root).unwrap_or(path);

        normalize_logical_path(logical_path)
    }

    /// Return the logical path for one file id when known.
    pub fn logical_path_by_file_id(&self, file_id: FileId) -> Option<Arc<str>> {
        self.logical_path_by_file_id
            .get(&file_id)
            .map(|entry| Arc::clone(entry.value()))
    }

    /// Return the file origin for one file id when known.
    pub(crate) fn file_origin_by_file_id(&self, file_id: FileId) -> Option<FileOrigin> {
        self.file_origin_by_file_id
            .get(&file_id)
            .map(|entry| entry.clone())
    }

    /// Build one file id for one normalized logical path string.
    pub fn file_id_for_logical_path(&self, logical_path: &str) -> FileId {
        let logical_path = normalize_logical_path_str(logical_path);
        FileId::from_logical_str(&logical_path)
    }

    /// Build one file id for one workspace file path.
    pub fn file_id_for_workspace_path(&self, path: &Path) -> FileId {
        let logical_path = self.normalize_workspace_path(path);
        self.file_id_for_logical_path(&logical_path)
    }

    /// Return the synthetic root file id.
    pub fn root_file_id(&self) -> FileId {
        self.file_id_for_logical_path("<root>")
    }

    /// Intern one logical path for one file id.
    fn intern_logical_path_by_file_id(&self, file_id: FileId, logical_path: &str) {
        let logical_path = normalize_logical_path_str(logical_path);

        self.logical_path_by_file_id
            .entry(file_id)
            .or_insert_with(|| Arc::<str>::from(logical_path));
    }

    /// Intern one root file identity.
    pub(crate) fn intern_root_file_id(&self, logical_path: &str) -> FileId {
        let file_id = self.file_id_for_logical_path(logical_path);
        self.intern_logical_path_by_file_id(file_id, logical_path);
        self.file_origin_by_file_id
            .insert(file_id, FileOrigin::Root);
        file_id
    }

    /// Intern one workspace file identity.
    pub(crate) fn intern_workspace_file_id(&self, path: &Path) -> FileId {
        let logical_path = self.normalize_workspace_path(path);
        let file_id = self.intern_workspace_logical_file_id(&logical_path);
        self.file_origin_by_file_id.insert(
            file_id,
            FileOrigin::Workspace {
                path: path.to_path_buf(),
            },
        );
        file_id
    }

    /// Intern one workspace logical file identity.
    pub(crate) fn intern_workspace_logical_file_id(&self, logical_path: &str) -> FileId {
        let file_id = self.file_id_for_logical_path(logical_path);
        self.intern_logical_path_by_file_id(file_id, logical_path);
        self.file_origin_by_file_id.insert(
            file_id,
            FileOrigin::Workspace {
                path: self.workspace.root.join(logical_path),
            },
        );
        file_id
    }

    /// Intern one builtin file identity.
    pub(crate) fn intern_builtin_file_id(
        &self,
        logical_path: &str,
        module_path: &str,
        source: ModuleSource,
    ) -> FileId {
        let file_id = self.file_id_for_logical_path(logical_path);
        self.intern_logical_path_by_file_id(file_id, logical_path);
        self.file_origin_by_file_id.insert(
            file_id,
            FileOrigin::Builtin {
                module_path: module_path.to_string(),
                source,
            },
        );
        file_id
    }
}
