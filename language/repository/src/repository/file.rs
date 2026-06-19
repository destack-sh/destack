use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use destack_artifact::{ArtifactDirectoryEntry, ArtifactPathState};
use destack_core::{Treap, TreapRoot};
use destack_source::{ContentId, File, FileId, FileMetadata, FileType, PathExt, StringId, Uri};
use rustc_hash::FxHashSet;

use crate::DestackFile;
use crate::repository::{Repository, RepositoryError, Revision};

/// The logical path prefix for mounted dependency roots.
pub(crate) const MOUNT_PREFIX: &str = "mount:";

/// Repository-owned source state tables.
#[derive(Debug)]
pub(crate) struct Files {
    /// Shared file entry treap.
    pub(crate) entries: Treap<FileId, FileEntry>,
    /// Parsed file data by exact file content.
    pub(crate) cache: FileCache,
}

impl Files {
    /// Create empty source state tables.
    pub(crate) fn new() -> Self {
        Self {
            entries: Treap::new(),
            cache: FileCache::new(),
        }
    }
}

/// Parsed file data keyed by exact file content.
#[derive(Debug, Default)]
pub(crate) struct FileCache {
    /// The config parse result by exact content.
    pub(crate) destack_by_content_id: DashMap<ContentId, Result<Arc<DestackFile>, String>>,
}

impl FileCache {
    /// Create one empty file cache.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Drop entries for file contents that are no longer reachable.
    pub(crate) fn retain_file_contents(&self, reachable: &HashSet<ContentId>) {
        self.destack_by_content_id
            .retain(|content_id, _| reachable.contains(content_id));
    }
}

/// The file binding for one file in one revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub(crate) struct FileEntry {
    /// The logical path for this file in this revision.
    pub logical_path: StringId,
    /// The fixed content binding for this file.
    pub content_id: ContentId,
}

impl FileEntry {
    /// Build one loaded file entry.
    pub(crate) fn loaded(logical_path: StringId, content_id: ContentId) -> Self {
        Self {
            logical_path,
            content_id,
        }
    }
}

impl Repository {
    /// Normalize one logical path for one workspace file path.
    #[track_caller]
    pub fn logical_path(&self, path: &Path) -> String {
        // prefer the direct workspace relative path
        if let Ok(logical_path) = path.strip_prefix(&self.root) {
            return normalize_logical_path(logical_path.to_string_lossy());
        }

        // mounted dependency roots map under their declared names
        if let Some(logical_path) = self.mounted_logical_path(path) {
            return logical_path;
        }

        // retry through canonical paths to collapse host path aliases
        let file_system = self.file_system();
        if let Ok(canonical_root) = file_system.canonicalize(&self.root)
            && let Ok(canonical_path) = file_system.canonicalize(path)
            && let Ok(logical_path) = canonical_path.strip_prefix(&canonical_root)
        {
            return normalize_logical_path(logical_path.to_string_lossy());
        }

        normalize_logical_path(path.to_string_lossy())
    }

    /// Intern one normalized logical repository path.
    pub(crate) fn intern_logical_path(&self, logical_path: impl AsRef<str>) -> StringId {
        let logical_path = normalize_logical_path(logical_path);

        self.strings.intern(&logical_path)
    }

    /// Return one interned logical repository path as text.
    pub(crate) fn logical_path_text(&self, logical_path: StringId) -> &str {
        self.strings.get(logical_path)
    }

    /// Return the physical workspace path for one file entry.
    pub(crate) fn file_entry_path(&self, entry: &FileEntry) -> PathBuf {
        self.physical_path(self.logical_path_text(entry.logical_path))
    }

    /// Return the physical path for one logical repository path.
    pub fn physical_path(&self, logical_path: &str) -> PathBuf {
        if let Some((mount, relative)) = split_mounted_path(logical_path)
            && let Some(base) = self.mounts.get(mount)
        {
            return base.join(relative);
        }

        self.root.join(logical_path)
    }

    /// Register one named dependency mount.
    pub fn add_mount(&self, name: &str, base: PathBuf) -> Result<(), RepositoryError> {
        let file_system = self.file_system();
        let base = file_system.canonicalize(&base).unwrap_or(base);
        if let Some(existing) = self.mounts.get(name) {
            if *existing != base {
                return Err(RepositoryError::MountConflict {
                    name: name.to_string(),
                    existing: existing.clone(),
                    base,
                });
            }

            return Ok(());
        }
        self.mounts.insert(name.to_string(), base);

        Ok(())
    }

    /// Return the mounted logical path for one physical path.
    fn mounted_logical_path(&self, path: &Path) -> Option<String> {
        let file_system = self.file_system();
        for entry in self.mounts.iter() {
            let relative = match path.strip_prefix(entry.value()) {
                Ok(relative) => relative.to_path_buf(),
                Err(_) => {
                    let Ok(canonical) = file_system.canonicalize(path) else {
                        continue;
                    };
                    let Ok(relative) = canonical.strip_prefix(entry.value()) else {
                        continue;
                    };

                    relative.to_path_buf()
                }
            };

            return Some(format!(
                "{MOUNT_PREFIX}{}/{}",
                entry.key(),
                normalize_logical_path(relative.to_string_lossy())
            ));
        }

        None
    }

    /// Build one file id for one workspace path.
    pub fn file_id(&self, path: &Path) -> FileId {
        // prefer immutable builtin files
        if let Some(file_id) = self.builtin.file_id_for_path(path) {
            return file_id;
        }

        // hash editable workspace paths
        let logical_path = self.logical_path(path);

        FileId::from_logical_str(&logical_path)
    }

    /// Return one source file for one revision and file id.
    pub fn file(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<Arc<File>>, RepositoryError> {
        // prefer immutable builtin files
        if let Some(builtin) = self.builtin.file(file_id) {
            return Ok(Some(Arc::new(builtin.file())));
        }

        // read editable revision files
        let revision = self.revision(revision)?;
        let Some(entry) = self.files.entries.get(revision.files(), &file_id) else {
            return Ok(None);
        };

        // assemble the physical source file
        let content = self.content(entry.content_id)?;
        let logical_path = self.logical_path_text(entry.logical_path);
        let path = self.file_entry_path(&entry);
        let uri = Uri::from_path(&path);
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| logical_path.to_string());
        let file_type = FileType::from_path_or_unknown(&path);
        let file = File::from_content(file_id, name, uri, Some(path), file_type, content);

        Ok(Some(Arc::new(file)))
    }

    /// Return one workspace path metadata for one revision and workspace path.
    pub fn file_metadata(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<FileMetadata>, RepositoryError> {
        // prefer immutable builtin files
        if let Some(builtin) = self.builtin.file_for_path(path) {
            return Ok(Some(builtin.metadata()));
        }

        // read editable file metadata
        let file_id = self.file_id(path);
        if let Some(file) = self.file(revision, file_id)? {
            return Ok(Some(FileMetadata::new(
                true,
                false,
                false,
                file.len as u64,
                None,
            )));
        }

        let revision_state = self.revision(revision)?;
        let revision_cache = revision_state.cache();
        let normalized_path = path.normalize();
        let directory_paths = revision_cache
            .directory_paths
            .get_or_init(|| Arc::new(self.directory_paths_for_files(revision_state.files())));

        // directory metadata
        if directory_paths.contains(&normalized_path) {
            return Ok(Some(FileMetadata::new(false, true, false, 0, None)));
        }

        Ok(None)
    }

    /// Return one file content identity from one revision.
    pub fn file_content_id(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<ContentId>, RepositoryError> {
        // prefer immutable builtin files
        if let Some(builtin) = self.builtin.file(file_id) {
            return Ok(Some(builtin.content_id()));
        }

        // read editable revision files
        let revision = self.revision(revision)?;
        let content_id = self
            .files
            .entries
            .get(revision.files(), &file_id)
            .map(|entry| entry.content_id);

        Ok(content_id)
    }

    /// Return the logical path for one file in one revision.
    pub fn file_logical_path(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<String>, RepositoryError> {
        // prefer immutable builtin files
        if let Some(builtin) = self.builtin.file(file_id) {
            return Ok(Some(builtin.uri.to_string()));
        }

        // read editable revision files
        let revision = self.revision(revision)?;
        let logical_path = self
            .files
            .entries
            .get(revision.files(), &file_id)
            .map(|entry| self.logical_path_text(entry.logical_path).to_string());

        Ok(logical_path)
    }

    /// Return the current source path state for one artifact source path.
    pub(crate) fn source_path_state(
        &self,
        revision: Revision,
        path: StringId,
    ) -> Result<ArtifactPathState, RepositoryError> {
        let path = self.string_pool().get(path);
        let path = self.physical_path(path);
        let state = match self.file_metadata(revision, &path)? {
            Some(metadata) if metadata.is_file => ArtifactPathState::File,
            Some(metadata) if metadata.is_directory => ArtifactPathState::Directory,
            Some(metadata) if metadata.is_symlink => ArtifactPathState::Symlink,
            Some(_metadata) => ArtifactPathState::Other,
            None => ArtifactPathState::Missing,
        };

        Ok(state)
    }

    /// Return the current direct source entries for one artifact directory path.
    pub(crate) fn source_directory_entries(
        &self,
        revision: Revision,
        directory: StringId,
    ) -> Result<Vec<ArtifactDirectoryEntry>, RepositoryError> {
        let revision = self.revision(revision)?;
        let directory = self.string_pool().get(directory);
        let mut entries = Vec::new();

        // collect editable source paths
        self.files
            .entries
            .visit(revision.files(), &mut |_file_id, entry| {
                let path = self.logical_path_text(entry.logical_path);
                if let Some(entry) = self.source_directory_entry(directory, path) {
                    entries.push(entry);
                }
            });

        // collect builtin source paths
        for builtin in self.builtin.files() {
            if let Some(entry) = self.source_directory_entry(directory, builtin.uri) {
                entries.push(entry);
            }
        }

        entries.sort_unstable();
        entries.dedup();

        Ok(entries)
    }

    /// Return a direct child entry when one source path is inside one directory.
    fn source_directory_entry(
        &self,
        directory: &str,
        path: &str,
    ) -> Option<ArtifactDirectoryEntry> {
        let path = normalize_logical_path(path);
        let entry_path = direct_child_path(directory, &path)?;
        let entry = self.intern_logical_path(&entry_path);
        let state = if entry_path == path {
            ArtifactPathState::File
        } else {
            ArtifactPathState::Directory
        };

        Some(ArtifactDirectoryEntry::new(entry, state))
    }

    /// Return editable file ids and interned logical paths for one revision.
    pub fn editable_file_logical_paths(
        &self,
        revision: Revision,
    ) -> Result<Vec<(FileId, StringId)>, RepositoryError> {
        let revision = self.revision(revision)?;
        let mut logical_paths = Vec::new();

        // collect editable file paths from the revision root
        self.files
            .entries
            .visit(revision.files(), &mut |file_id, entry| {
                logical_paths.push((*file_id, entry.logical_path));
            });

        Ok(logical_paths)
    }

    /// Return the file ids visible in one revision.
    pub fn file_ids(&self, revision: Revision) -> Result<Vec<FileId>, RepositoryError> {
        // start with editable revision files
        let revision = self.revision(revision)?;
        let mut file_ids = Vec::new();
        self.files
            .entries
            .visit(revision.files(), &mut |file_id, _entry| {
                file_ids.push(*file_id)
            });

        // append immutable builtin files
        file_ids.extend(self.builtin.files().iter().map(|builtin| builtin.file_id()));
        file_ids.sort_unstable();
        file_ids.dedup();

        Ok(file_ids)
    }

    /// Build the workspace directory set for one revision.
    fn directory_paths_for_files(&self, files: TreapRoot) -> FxHashSet<PathBuf> {
        let mut directories = FxHashSet::default();
        directories.insert(self.root.normalize());

        // physical workspace directories
        self.files.entries.visit(files, &mut |_file_id, entry| {
            let path = self.file_entry_path(entry);

            let mut current = path.parent().map(Path::to_path_buf);
            while let Some(directory) = current {
                let directory = directory.normalize();
                if !directory.starts_with(&self.root) {
                    break;
                }

                directories.insert(directory.clone());

                if directory == self.root {
                    break;
                }

                current = directory.parent().map(Path::to_path_buf);
            }
        });

        directories
    }
}

/// Split one logical path into its mount name and relative remainder.
pub(crate) fn split_mounted_path(logical_path: &str) -> Option<(&str, &str)> {
    let mounted = logical_path.strip_prefix(MOUNT_PREFIX)?;

    mounted.split_once('/').or(Some((mounted, "")))
}

/// Normalize one logical file path.
pub(crate) fn normalize_logical_path(value: impl AsRef<str>) -> String {
    let value = value.as_ref();

    value.replace('\\', "/")
}

/// Return the direct child path when one source path is inside one directory.
fn direct_child_path(directory: &str, path: &str) -> Option<String> {
    let relative = if directory.is_empty() {
        path
    } else {
        path.strip_prefix(directory)?.strip_prefix('/')?
    };

    if let Some((child, _descendant)) = relative.split_once('/') {
        if directory.is_empty() {
            Some(child.to_string())
        } else {
            Some(format!("{directory}/{child}"))
        }
    } else {
        Some(path.to_string())
    }
}
