use std::path::Path;

use serde::de::DeserializeOwned;

use crate::{ArtifactCache, ArtifactCacheError};

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::collections::HashMap;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::path::PathBuf;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::time::{Duration, SystemTime};

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use destack_source::{FileMetadata, FileSystem, PhysicalFileSystem};

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use super::ArtifactCacheLockMode;
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use crate::ArtifactCacheManifest;

/// Minimum interval between automatic artifact cache collections.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
const COLLECTION_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
/// Persistent marker for the last completed artifact cache collection.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
const LAST_COLLECTION_FILE: &str = "last-collection";

/// One completed artifact cache collection.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ArtifactCacheCollection {
    /// Cache bytes present before collection.
    before_bytes: u64,
    /// Cache bytes present after collection.
    after_bytes: u64,
    /// Files removed during collection.
    removed_files: u64,
}

/// One cached file considered during collection.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
#[derive(Debug)]
struct CachedFile {
    /// Physical cache path.
    path: PathBuf,
    /// Exact file size.
    bytes: u64,
    /// Last file modification.
    modified_at: SystemTime,
}

/// One repository manifest and its selected packs.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
#[derive(Debug)]
struct CachedManifest {
    /// Physical manifest file.
    file: CachedFile,
    /// Canonical repository path.
    repository: PathBuf,
    /// Physical paths of selected packs.
    packs: Box<[PathBuf]>,
}

/// One obsolete build directory considered as an indivisible unit.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
#[derive(Debug)]
struct CachedBuild {
    /// Physical build directory.
    path: PathBuf,
    /// Total bytes below the directory.
    bytes: u64,
    /// Number of files below the directory.
    files: u64,
    /// Newest modification in the directory tree.
    modified_at: SystemTime,
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
impl ArtifactCache {
    /// Collect this cache when its automatic collection interval has elapsed.
    pub fn collect_if_due<R: DeserializeOwned>(
        &self,
        retained_repository: &Path,
    ) -> Result<(), ArtifactCacheError> {
        let _lock = self.lock(ArtifactCacheLockMode::Exclusive)?;
        let now = SystemTime::now();
        let last_collection = self.last_collection()?;

        // skip one recently completed collection
        let is_recent = last_collection.is_some_and(|last_collection| {
            match now.duration_since(last_collection) {
                Ok(elapsed) => elapsed < COLLECTION_INTERVAL,
                Err(_) => true,
            }
        });
        if is_recent {
            return Ok(());
        }

        // protect files newer than the collection cutoff
        let eligible_before = match last_collection {
            Some(last_collection) => last_collection,
            None => now.checked_sub(COLLECTION_INTERVAL).ok_or_else(|| {
                ArtifactCacheError::Internal(
                    "system time precedes the collection interval".to_string(),
                )
            })?,
        };

        // collect eligible records and publish the completed timestamp
        self.collect::<R>(retained_repository, eligible_before)?;
        self.write_last_collection()?;

        Ok(())
    }

    /// Collect unreachable and over-budget artifact cache records.
    fn collect<R: DeserializeOwned>(
        &self,
        retained_repository: &Path,
        eligible_before: SystemTime,
    ) -> Result<ArtifactCacheCollection, ArtifactCacheError> {
        // scan current records and opaque obsolete builds
        let mut obsolete = self.obsolete_builds()?;
        self.validate_current_build()?;
        let packs = self.cached_files(&self.build_directory.join("packs"), "pack")?;
        let mut pack_files = packs
            .iter()
            .map(|pack| (pack.path.as_path(), (pack.bytes, pack.modified_at)))
            .collect::<HashMap<_, _>>();
        let mut manifests = self.cached_manifests::<R>(&pack_files)?;

        // total every measured cache file
        let obsolete_bytes = obsolete.iter().map(|build| build.bytes).sum::<u64>();
        let current_pack_bytes = pack_files.values().map(|(bytes, _)| bytes).sum::<u64>();
        let current_manifest_bytes = manifests
            .iter()
            .map(|manifest| manifest.file.bytes)
            .sum::<u64>();
        let before_bytes = obsolete_bytes + current_pack_bytes + current_manifest_bytes;
        let mut after_bytes = before_bytes;
        let mut removed_files = 0;

        // remove eligible obsolete builds before current repository selections
        obsolete.retain(|build| build.modified_at <= eligible_before);
        obsolete.sort_unstable_by(|left, right| {
            left.modified_at
                .cmp(&right.modified_at)
                .then_with(|| left.path.cmp(&right.path))
        });
        if let Some(maximum_bytes) = self.maximum_bytes {
            for build in obsolete {
                if after_bytes <= maximum_bytes {
                    break;
                }
                Self::remove_path(&build.path)?;
                after_bytes -= build.bytes;
                removed_files += build.files;
            }
        }

        // index every pack selected by current manifests
        manifests.sort_unstable_by(|left, right| {
            left.file
                .modified_at
                .cmp(&right.file.modified_at)
                .then_with(|| left.file.path.cmp(&right.file.path))
        });
        let mut references = HashMap::<&Path, usize>::new();
        for manifest in &manifests {
            for pack in &manifest.packs {
                *references.entry(pack.as_path()).or_default() += 1;
            }
        }

        // remove packs unreferenced since the preceding collection
        for pack in &packs {
            let is_referenced = references.contains_key(pack.path.as_path());
            if !is_referenced && pack.modified_at <= eligible_before {
                Self::remove_file(&pack.path)?;
                after_bytes -= pack.bytes;
                removed_files += 1;
                pack_files.remove(pack.path.as_path());
            }
        }

        // evict the least recently used repository selections when over budget
        if let Some(maximum_bytes) = self.maximum_bytes {
            for manifest in &manifests {
                let is_retained = manifest.repository == retained_repository;
                let is_eligible = manifest.file.modified_at <= eligible_before;
                if after_bytes <= maximum_bytes || is_retained || !is_eligible {
                    continue;
                }

                Self::remove_file(&manifest.file.path)?;
                after_bytes -= manifest.file.bytes;
                removed_files += 1;

                // remove packs released by this last selecting manifest
                for pack in &manifest.packs {
                    let Some(count) = references.get_mut(pack.as_path()) else {
                        return Err(ArtifactCacheError::Internal(format!(
                            "manifest references unindexed pack {}",
                            pack.display()
                        )));
                    };
                    *count -= 1;
                    if *count == 0 {
                        let Some(&(bytes, modified_at)) = pack_files.get(pack.as_path()) else {
                            return Err(ArtifactCacheError::Internal(format!(
                                "manifest references unindexed pack {}",
                                pack.display()
                            )));
                        };
                        if modified_at <= eligible_before {
                            Self::remove_file(pack)?;
                            after_bytes -= bytes;
                            removed_files += 1;
                            pack_files.remove(pack.as_path());
                        }
                    }
                }
            }
        }

        Ok(ArtifactCacheCollection {
            before_bytes,
            after_bytes,
            removed_files,
        })
    }

    /// Read every repository manifest stored for the current build.
    fn cached_manifests<R: DeserializeOwned>(
        &self,
        pack_files: &HashMap<&Path, (u64, SystemTime)>,
    ) -> Result<Vec<CachedManifest>, ArtifactCacheError> {
        let directory = self.build_directory.join("manifests");
        let files = self.cached_files(&directory, "manifest")?;
        let mut manifests = Vec::with_capacity(files.len());
        for file in files {
            // decode and validate one current manifest
            let bytes = Self::read_file(&file.path)?;
            let manifest: ArtifactCacheManifest<R> = destack_serde::from_slice(&bytes)
                .map_err(ArtifactCacheError::from)
                .map_err(|error| error.record(&file.path))?;
            manifest
                .validate(self.build_id())
                .map_err(|error| error.record(&file.path))?;

            // require the repository-derived manifest path
            let expected = self.manifest_path(manifest.repository());
            if expected != file.path {
                let error = ArtifactCacheError::Invalid(
                    "manifest path does not match its repository".to_string(),
                );

                return Err(error.record(&file.path));
            }

            // require every selected immutable pack
            let packs = manifest
                .packs
                .iter()
                .map(|selected| self.pack_path(selected.version))
                .collect::<Box<[_]>>();
            for pack in &packs {
                if !pack_files.contains_key(pack.as_path()) {
                    let error = ArtifactCacheError::Invalid(format!(
                        "manifest references missing pack {}",
                        pack.display()
                    ));

                    return Err(error.record(&file.path));
                }
            }

            // retain the decoded repository selection
            manifests.push(CachedManifest {
                file,
                repository: manifest.repository().to_path_buf(),
                packs,
            });
        }

        Ok(manifests)
    }

    /// Read cache files with one exact extension.
    fn cached_files(
        &self,
        directory: &Path,
        extension: &str,
    ) -> Result<Vec<CachedFile>, ArtifactCacheError> {
        let entries = Self::read_directory(directory)?;
        let mut files = Vec::new();
        for path in entries {
            let metadata = Self::metadata(&path)?;
            let has_extension = path.extension().is_some_and(|value| value == extension);
            if !metadata.is_file || !has_extension {
                let error = ArtifactCacheError::Invalid(format!(
                    "unexpected cache entry, expected a .{extension} file"
                ));

                return Err(error.record(&path));
            }

            // retain exact file size and recency
            let modified_at = metadata.modified_at.ok_or_else(|| {
                ArtifactCacheError::Invalid("cache file has no modification time".to_string())
                    .record(&path)
            })?;
            files.push(CachedFile {
                path,
                bytes: metadata.size_bytes,
                modified_at,
            });
        }

        Ok(files)
    }

    /// Measure every obsolete build directory without decoding its format.
    fn obsolete_builds(&self) -> Result<Vec<CachedBuild>, ArtifactCacheError> {
        let directory = self.directory.join("builds");
        let entries = Self::read_directory(&directory)?;
        let mut builds = Vec::new();
        for path in entries {
            let metadata = Self::metadata(&path)?;
            if !metadata.is_directory {
                let error = ArtifactCacheError::Invalid(
                    "unexpected cache entry, expected a build directory".to_string(),
                );

                return Err(error.record(&path));
            }

            // measure each opaque build directory
            if path != self.build_directory {
                let (bytes, files, modified_at) = Self::measure_directory(&path)?;
                builds.push(CachedBuild {
                    path,
                    bytes,
                    files,
                    modified_at,
                });
            }
        }

        Ok(builds)
    }

    /// Validate the exact current build directory entries.
    fn validate_current_build(&self) -> Result<(), ArtifactCacheError> {
        let manifests = self.build_directory.join("manifests");
        let packs = self.build_directory.join("packs");
        let expected = [manifests, packs];
        let entries = Self::read_directory(&self.build_directory)?;
        if entries != expected {
            let error = ArtifactCacheError::Invalid(
                "current build must contain exactly manifests and packs directories".to_string(),
            );

            return Err(error.record(&self.build_directory));
        }

        Ok(())
    }

    /// Measure every file below one cache directory.
    fn measure_directory(path: &Path) -> Result<(u64, u64, SystemTime), ArtifactCacheError> {
        let directory = Self::metadata(path)?;
        let modified_at = directory.modified_at.ok_or_else(|| {
            ArtifactCacheError::Invalid("cache directory has no modification time".to_string())
                .record(path)
        })?;
        let mut bytes = 0_u64;
        let mut files = 0_u64;
        let mut modified_at = modified_at;
        let mut pending = vec![path.to_path_buf()];
        while let Some(directory) = pending.pop() {
            // measure every descendant exactly once
            for entry in Self::read_directory(&directory)? {
                let metadata = Self::metadata(&entry)?;
                if metadata.is_directory {
                    pending.push(entry);
                } else if metadata.is_file {
                    let file_modified = metadata.modified_at.ok_or_else(|| {
                        ArtifactCacheError::Invalid(
                            "cache file has no modification time".to_string(),
                        )
                        .record(&entry)
                    })?;
                    bytes += metadata.size_bytes;
                    files += 1;
                    modified_at = modified_at.max(file_modified);
                } else {
                    let error = ArtifactCacheError::Invalid(
                        "unexpected cache entry, expected a file or directory".to_string(),
                    );

                    return Err(error.record(&entry));
                }
            }
        }

        Ok((bytes, files, modified_at))
    }

    /// Read the preceding collection timestamp when present.
    fn last_collection(&self) -> Result<Option<SystemTime>, ArtifactCacheError> {
        let path = self.directory.join(LAST_COLLECTION_FILE);
        if !Self::exists(&path)? {
            return Ok(None);
        }

        // read collection recency directly from its marker
        let metadata = Self::metadata(&path)?;
        if !metadata.is_file {
            let error =
                ArtifactCacheError::Invalid("last collection marker must be a file".to_string());

            return Err(error.record(&path));
        }
        metadata.modified_at.map(Some).ok_or_else(|| {
            ArtifactCacheError::Invalid(
                "last collection marker has no modification time".to_string(),
            )
            .record(&path)
        })
    }

    /// Atomically record one completed collection.
    fn write_last_collection(&self) -> Result<(), ArtifactCacheError> {
        let path = self.directory.join(LAST_COLLECTION_FILE);

        Self::write_file(&path, &[])
    }

    /// Read one directory in stable path order.
    fn read_directory(path: &Path) -> Result<Vec<PathBuf>, ArtifactCacheError> {
        let mut entries =
            PhysicalFileSystem
                .read_dir(path)
                .map_err(|error| ArtifactCacheError::FileSystem {
                    operation: "read directory",
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })?;
        entries.sort_unstable();

        Ok(entries)
    }

    /// Read metadata for one cache path.
    fn metadata(path: &Path) -> Result<FileMetadata, ArtifactCacheError> {
        PhysicalFileSystem
            .symlink_metadata(path)
            .map_err(|error| ArtifactCacheError::FileSystem {
                operation: "stat",
                path: path.to_path_buf(),
                message: error.to_string(),
            })
    }

    /// Remove one cache file or directory tree.
    fn remove_path(path: &Path) -> Result<(), ArtifactCacheError> {
        PhysicalFileSystem
            .remove_path(path)
            .map(|_| ())
            .map_err(|error| ArtifactCacheError::FileSystem {
                operation: "remove",
                path: path.to_path_buf(),
                message: error.to_string(),
            })
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl ArtifactCache {
    /// Skip persistent cache collection on bare WebAssembly.
    pub fn collect_if_due<R: DeserializeOwned>(
        &self,
        _retained_repository: &Path,
    ) -> Result<(), ArtifactCacheError> {
        Ok(())
    }
}

#[cfg(all(test, not(all(target_arch = "wasm32", target_os = "unknown"))))]
mod tests {
    use destack_core::{BlobStore, StringPool};
    use destack_source::TemporaryPhysicalFileSystem;

    use super::*;
    use crate::{ArtifactPack, ArtifactPackReference, ArtifactPackVersion, BuildId};

    /// Retain shared packs until their last repository manifest is evicted.
    #[test]
    fn test_collect_retains_shared_packs() {
        let files = TemporaryPhysicalFileSystem::new_with_prefix("artifact-cache-collection");
        let cache = ArtifactCache::open(BuildId::test(), files.root(), Some(0))
            .expect("open artifact cache");
        let first_repository = files.path_for("first");
        let retained_repository = files.path_for("retained");
        let version = ArtifactPackVersion::new([]);

        // publish two repository manifests selecting one valid shared pack
        let publication = cache.publish().expect("begin cache publication");
        let bytes = ArtifactPack::encode(
            BuildId::test(),
            None,
            &[],
            &StringPool::new(),
            &BlobStore::new(),
        )
        .expect("encode empty artifact pack");
        let checksum = publication
            .write_pack(version, bytes)
            .expect("publish shared pack");
        let packs = [ArtifactPackReference::new(None, version, checksum)];
        let first = ArtifactCacheManifest::new(BuildId::test(), &first_repository, 1_u64, packs);
        publication
            .write_manifest(&first_repository, &first)
            .expect("publish first manifest");
        let retained =
            ArtifactCacheManifest::new(BuildId::test(), &retained_repository, 1_u64, packs);
        publication
            .write_manifest(&retained_repository, &retained)
            .expect("publish retained manifest");
        drop(publication);

        // evict the unretained repository while preserving its shared pack
        let collected = cache
            .collect::<u64>(&retained_repository, SystemTime::now())
            .expect("collect unretained repository");
        assert_eq!(collected.removed_files, 1);
        assert!(!cache.manifest_path(&first_repository).exists());
        assert!(cache.manifest_path(&retained_repository).exists());
        assert!(cache.pack_path(version).exists());
        assert!(
            cache
                .load::<u64>(&retained_repository, 1)
                .expect("load retained repository")
                .is_some()
        );

        // release the final manifest and its now-unreferenced pack
        let collected = cache
            .collect::<u64>(Path::new("unretained"), SystemTime::now())
            .expect("collect final repository");
        assert_eq!(collected.removed_files, 2);
        assert_eq!(collected.after_bytes, 0);
        assert!(!cache.manifest_path(&retained_repository).exists());
        assert!(!cache.pack_path(version).exists());
    }

    /// Remove an eligible obsolete build as one opaque unit.
    #[test]
    fn test_collect_removes_obsolete_build() {
        let files = TemporaryPhysicalFileSystem::new_with_prefix("artifact-cache-build");
        let obsolete_build = BuildId::new([0xff; 16]);
        let obsolete_path = PathBuf::from("builds")
            .join(obsolete_build.to_string())
            .join("opaque-record");
        files
            .write_bytes(&obsolete_path, b"obsolete")
            .expect("write obsolete build record");
        let cache = ArtifactCache::open(BuildId::test(), files.root(), Some(0))
            .expect("open current artifact cache");

        // evict the complete obsolete build without decoding its format
        let collected = cache
            .collect::<u64>(Path::new("retained"), SystemTime::now())
            .expect("collect obsolete build");
        assert_eq!(collected.before_bytes, 8);
        assert_eq!(collected.after_bytes, 0);
        assert_eq!(collected.removed_files, 1);
        assert!(!files.root().join(obsolete_path).exists());
    }
}
