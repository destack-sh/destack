use std::path::Path;

use destack_source::FileId;

use crate::repository::{FileOrigin, Repository, SyntheticFileKind};

const WORKSPACE_FILE_ORIGIN_TAG: u8 = 0;
const BUILTIN_FILE_ORIGIN_TAG: u8 = 1;
const SYNTHETIC_FILE_ORIGIN_TAG: u8 = 2;
const NAMED_SYNTHETIC_FILE_KIND_TAG: u8 = 0;
const ROOT_SYNTHETIC_FILE_KIND_TAG: u8 = 1;

/// Normalize one logical file path string.
pub(crate) fn normalize_logical_path_str(value: &str) -> String {
    value.replace('\\', "/")
}

/// Normalize one logical file path.
pub(crate) fn normalize_logical_path(path: &Path) -> String {
    normalize_logical_path_str(&path.to_string_lossy())
}

impl Repository {
    /// Build one structured file id payload for one file origin.
    fn file_id_bytes_for_origin(&self, origin: &FileOrigin) -> Vec<u8> {
        let mut bytes = Vec::new();

        // origin namespace
        match origin {
            FileOrigin::Workspace { logical_path } => {
                bytes.push(WORKSPACE_FILE_ORIGIN_TAG);
                Self::push_normalized_logical_path(&mut bytes, logical_path);
            }
            FileOrigin::Builtin { logical_path } => {
                bytes.push(BUILTIN_FILE_ORIGIN_TAG);
                Self::push_normalized_logical_path(&mut bytes, logical_path);
            }
            FileOrigin::Synthetic {
                logical_path,
                kind,
                file_type,
            } => {
                bytes.push(SYNTHETIC_FILE_ORIGIN_TAG);
                bytes.push(Self::synthetic_file_kind_tag(*kind));
                bytes.push(*file_type as u8);
                Self::push_normalized_logical_path(&mut bytes, logical_path);
            }
        }

        bytes
    }

    /// Push one normalized logical path into one file id payload.
    fn push_normalized_logical_path(bytes: &mut Vec<u8>, logical_path: &str) {
        let logical_path = normalize_logical_path_str(logical_path);
        bytes.extend_from_slice(logical_path.as_bytes());
    }

    /// Return the stable file id tag for one synthetic file kind.
    fn synthetic_file_kind_tag(kind: SyntheticFileKind) -> u8 {
        match kind {
            SyntheticFileKind::Named => NAMED_SYNTHETIC_FILE_KIND_TAG,
            SyntheticFileKind::Root => ROOT_SYNTHETIC_FILE_KIND_TAG,
        }
    }

    /// Normalize one logical path for one workspace file path.
    pub fn normalize_workspace_path(&self, path: &Path) -> String {
        // prefer the direct workspace-relative path
        if let Ok(logical_path) = path.strip_prefix(&self.root) {
            return normalize_logical_path(logical_path);
        }

        // retry through canonical paths to collapse host path aliases like /var and /private/var
        if let Ok(canonical_root) = self.fs.canonicalize(&self.root)
            && let Ok(canonical_path) = self.fs.canonicalize(path)
            && let Ok(logical_path) = canonical_path.strip_prefix(&canonical_root)
        {
            return normalize_logical_path(logical_path);
        }

        let logical_path = path;
        normalize_logical_path(logical_path)
    }

    /// Build one file id for one normalized logical path string.
    pub fn file_id_for_logical_path(&self, logical_path: &str) -> FileId {
        let logical_path = normalize_logical_path_str(logical_path);
        FileId::from_logical_str(&logical_path)
    }

    /// Build one file id for one explicit file origin.
    pub fn file_id_for_origin(&self, origin: &FileOrigin) -> FileId {
        let bytes = self.file_id_bytes_for_origin(origin);
        FileId::from_origin_bytes(&bytes)
    }

    /// Build one file id for one workspace file path.
    pub fn file_id_for_workspace_path(&self, path: &Path) -> FileId {
        let logical_path = self.normalize_workspace_path(path);
        self.file_id_for_logical_path(&logical_path)
    }

    /// Return the synthetic root file id.
    pub fn root_file_id(&self) -> FileId {
        self.file_id_for_origin(&FileOrigin::root())
    }
}
