use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use dashmap::DashMap;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use tspp_core::{Blob, BlobId};
use tspp_serde::Reflect;
use tspp_source as source;
use tspp_source::{FileId, FileMetadata, FileType, PathExt, StringId, Uri};

use crate::DestackFile;
use crate::repository::{Repository, RepositoryError, Revision};

/// The logical path prefix for mounted dependency roots.
pub(crate) const MOUNT_PREFIX: &str = "mount:";

/// One file bound into an immutable repository revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct File {
    /// Path-derived file identity.
    pub id: FileId,
    /// Normalized repository-relative path.
    pub path: String,
    /// Exact content Blob.
    pub blob: Blob,
}

/// The file binding for one file in one revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct FileEntry {
    /// The logical path for this file in this revision.
    pub logical_path: StringId,
    /// The fixed content binding for this file.
    pub blob: Blob,
}

impl FileEntry {
    /// Build one loaded file entry.
    pub(crate) fn new(logical_path: StringId, blob: Blob) -> Self {
        Self { logical_path, blob }
    }
}

/// Loaded and parsed file state keyed by exact Blobs.
#[derive(Debug, Default)]
pub(crate) struct FileCache {
    /// Loaded source files by identity and exact Blob.
    pub(crate) files: DashMap<(FileId, BlobId), Arc<source::File>>,
    /// Parsed TS++ files by path and exact Blob.
    pub(crate) destack: DashMap<(FileId, BlobId), Result<Arc<DestackFile>, String>>,
}

impl FileCache {
    /// Retain entries backed by reachable Blobs.
    pub(crate) fn retain(&self, reachable: &HashSet<BlobId>) {
        self.files
            .retain(|(_file, blob), _| reachable.contains(blob));
        self.destack
            .retain(|(_file, blob), _| reachable.contains(blob));
    }
}

impl Repository {
    /// List files bound into one immutable revision in path order.
    pub fn files(&self, revision: Revision) -> Result<Vec<File>, RepositoryError> {
        let files = self.file_entries(revision)?;
        let mut files = files
            .iter()
            .map(|(id, entry)| File {
                id: *id,
                path: self.logical_path_text(entry.logical_path).to_string(),
                blob: entry.blob,
            })
            .collect::<Vec<_>>();
        files.sort_unstable_by(|left, right| left.path.cmp(&right.path));

        Ok(files)
    }

    /// Return the dense file entries for one revision.
    pub(crate) fn file_entries(
        &self,
        revision: Revision,
    ) -> Result<Arc<[(FileId, FileEntry)]>, RepositoryError> {
        let revision = self.revision(revision)?;
        let cache = revision.cache();
        let files = cache.files.get_or_init(|| {
            let files = self.file_tree.entries(revision.files());

            Arc::from(files.into_boxed_slice())
        });

        Ok(files.clone())
    }

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

        self.string_pool().intern(&logical_path)
    }

    /// Return one interned logical repository path as text.
    pub(crate) fn logical_path_text(&self, logical_path: StringId) -> &str {
        self.string_pool().get(logical_path)
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
    pub(crate) fn add_mount(&self, name: &str, base: PathBuf) -> Result<(), RepositoryError> {
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
        // resolve embedded Builtin source URIs before authored paths
        if let Some(file_id) = self.embedded_builtin().file_id_for_path(path) {
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
    ) -> Result<Option<Arc<source::File>>, RepositoryError> {
        // resolve embedded Builtin FileIds from their immutable table
        if let Some(builtin) = self.embedded_builtin().file(file_id) {
            return Ok(Some(builtin.clone()));
        }

        // read editable revision files
        let revision = self.revision(revision)?;
        let Some(entry) = self.file_tree.get(revision.files(), &file_id) else {
            return Ok(None);
        };

        // reuse retained Blob memory and line offsets for this exact file binding
        let cache_key = (file_id, entry.blob.id);
        if let Some(file) = self.file_cache.files.get(&cache_key) {
            return Ok(Some(file.clone()));
        }

        // assemble the physical source file
        let memory = self
            .blob_store()
            .open(entry.blob)
            .map_err(|error| RepositoryError::Blob {
                message: error.to_string(),
            })?;
        let path = self.file_entry_path(&entry);
        let uri = Uri::from_path(&path);
        let name_and_type = path.file_name().zip(FileType::from_path(&path));
        let Some((name, file_type)) = name_and_type else {
            return Err(RepositoryError::InvalidFile {
                file: file_id,
                message: format!("repository path must name a file: {}", path.display()),
            });
        };
        let name = name.to_string_lossy().into_owned();
        let file = source::File::from_blob(file_id, name, uri, Some(path), file_type, memory)
            .map_err(|error| RepositoryError::InvalidFile {
                file: file_id,
                message: error.to_string(),
            })?;

        // publish one shared File when concurrent readers loaded the same binding
        let file = Arc::new(file);
        let file = self
            .file_cache
            .files
            .entry(cache_key)
            .or_insert(file)
            .clone();

        Ok(Some(file))
    }

    /// Return one workspace path metadata for one revision and workspace path.
    pub fn file_metadata(
        &self,
        revision: Revision,
        path: &Path,
    ) -> Result<Option<FileMetadata>, RepositoryError> {
        // resolve embedded Builtin source URIs before authored paths
        if let Some(builtin) = self.embedded_builtin().builtin_file_for_path(path) {
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
        let files = self.file_entries(revision)?;
        let normalized_path = path.normalize();
        let directory_paths = revision_cache
            .directory_paths
            .get_or_init(|| Arc::new(self.directory_paths_for_files(files.as_ref())));

        // directory metadata
        if directory_paths.contains(&normalized_path) {
            return Ok(Some(FileMetadata::new(false, true, false, 0, None)));
        }

        Ok(None)
    }

    /// Return one file Blob from one revision.
    pub fn file_blob(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<Blob>, RepositoryError> {
        // resolve embedded Builtin FileIds from their immutable table
        if let Some(blob) = self.embedded_builtin().builtin_blob(file_id) {
            return Ok(Some(blob));
        }

        // read editable revision files
        let revision = self.revision(revision)?;
        let blob = self
            .file_tree
            .get(revision.files(), &file_id)
            .map(|entry| entry.blob);

        Ok(blob)
    }

    /// Return the logical path for one file in one revision.
    pub fn file_logical_path(
        &self,
        revision: Revision,
        file_id: FileId,
    ) -> Result<Option<String>, RepositoryError> {
        // resolve embedded Builtin FileIds from their immutable table
        if let Some(builtin) = self.embedded_builtin().builtin_file(file_id) {
            return Ok(Some(builtin.uri.to_string()));
        }

        // read editable revision files
        let revision = self.revision(revision)?;
        let logical_path = self
            .file_tree
            .get(revision.files(), &file_id)
            .map(|entry| self.logical_path_text(entry.logical_path).to_string());

        Ok(logical_path)
    }

    /// Return editable file ids and interned logical paths for one revision.
    pub fn editable_file_logical_paths(
        &self,
        revision: Revision,
    ) -> Result<Vec<(FileId, StringId)>, RepositoryError> {
        let files = self.file_entries(revision)?;
        let mut logical_paths = Vec::new();

        // collect editable file paths from the revision root
        for (file_id, entry) in files.iter().copied() {
            logical_paths.push((file_id, entry.logical_path));
        }

        Ok(logical_paths)
    }

    /// Return the file ids visible in one revision.
    pub fn file_ids(&self, revision: Revision) -> Result<Vec<FileId>, RepositoryError> {
        // start with editable revision files
        let mut file_ids = self
            .file_entries(revision)?
            .iter()
            .copied()
            .map(|(file_id, _entry)| file_id)
            .collect::<Vec<_>>();

        // include embedded files only when no authored Builtin Package replaces them
        let package = self.builtin_package(revision)?;
        if package.path.is_none() {
            file_ids.extend(
                self.embedded_builtin()
                    .files()
                    .iter()
                    .map(|builtin| builtin.file_id()),
            );
        }
        file_ids.sort_unstable();
        file_ids.dedup();

        Ok(file_ids)
    }

    /// Build the workspace directory set for one revision.
    fn directory_paths_for_files(&self, files: &[(FileId, FileEntry)]) -> FxHashSet<PathBuf> {
        let mut directories = FxHashSet::default();
        directories.insert(self.root.normalize());

        // physical workspace directories
        for (_file_id, entry) in files {
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
        }

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
