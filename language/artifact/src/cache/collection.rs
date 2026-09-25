use std::path::Path;

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::ErrorKind,
    path::PathBuf,
    time::{Duration, SystemTime},
};

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::{ArtifactCache, ArtifactCacheError, BuildId};

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use tspp_source::{FileMetadata, FileSystem, PhysicalFileSystem};

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

/// Artifact cache usage for one set of builds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ArtifactCacheStats {
    /// Bytes retained by these builds.
    pub bytes: u64,
    /// Files retained by these builds.
    pub files: u64,
    /// Build directories in this set.
    pub builds: u64,
    /// Repository manifests retained by these builds.
    pub manifests: u64,
    /// Immutable packs retained by these builds.
    pub packs: u64,
}

impl ArtifactCacheStats {
    /// Include one measured build directory.
    fn include(&mut self, build: &CachedDirectory) {
        self.bytes += build.bytes;
        self.files += build.files;
        self.builds += 1;
        self.manifests += build.manifests;
        self.packs += build.packs;
    }
}

/// Cached build usage grouped by producer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ArtifactCacheUsage {
    /// Usage retained by the current Destack build.
    pub current_build: ArtifactCacheStats,
    /// Usage retained by other Destack builds.
    pub other_builds: ArtifactCacheStats,
    /// Usage retained by every Destack build.
    pub total: ArtifactCacheStats,
}

/// Records removed from one machine artifact cache.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct ArtifactCacheRemoval {
    /// Bytes removed from the cache.
    pub bytes: u64,
    /// Files removed from the cache.
    pub files: u64,
}

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

/// One cached file measured during collection.
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

/// One measured cache directory.
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
#[derive(Debug)]
struct CachedDirectory {
    /// Physical cache directory.
    path: PathBuf,
    /// Total bytes below the directory.
    bytes: u64,
    /// Number of files below the directory.
    files: u64,
    /// Repository manifests below the directory.
    manifests: u64,
    /// Immutable packs below the directory.
    packs: u64,
    /// Newest modification in the directory tree.
    modified_at: SystemTime,
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
impl ArtifactCache {
    /// Measure one machine cache without opening a build lease.
    pub fn measure(
        directory: &Path,
        build_id: BuildId,
    ) -> Result<ArtifactCacheUsage, ArtifactCacheError> {
        if !Self::exists(directory)? {
            return Ok(ArtifactCacheUsage::default());
        }

        // retain a stable cache view while measuring its records
        let _lock = Self::lock_directory(directory, ArtifactCacheLockMode::Shared)?;
        let builds_directory = directory.join("builds");

        // validate and measure machine cache metadata
        let mut usage = ArtifactCacheUsage::default();
        for path in Self::read_directory(directory)? {
            let name = path.file_name().and_then(|name| name.to_str());
            match name {
                Some("lock") => {
                    if !Self::metadata(&path)?.is_file {
                        return Err(Self::invalid_entry(&path, "a lock file"));
                    }
                }
                Some(LAST_COLLECTION_FILE) => {
                    let metadata = Self::metadata(&path)?;
                    if !metadata.is_file {
                        return Err(Self::invalid_entry(&path, "a collection marker file"));
                    }
                }
                Some("builds") => {
                    if !Self::metadata(&path)?.is_directory {
                        return Err(Self::invalid_entry(&path, "a builds directory"));
                    }
                }
                _ => return Err(Self::invalid_entry(&path, "a cache record")),
            }
        }

        // classify each opaque build directory
        if Self::exists(&builds_directory)? {
            let current = builds_directory.join(build_id.to_string());
            for path in Self::read_directory(&builds_directory)? {
                let build = Self::cached_directory(path)?;
                usage.total.include(&build);

                // attribute this directory to its exact build set
                if build.path == current {
                    usage.current_build.include(&build);
                } else {
                    usage.other_builds.include(&build);
                }
            }
        }

        Ok(usage)
    }

    /// Remove every record after requiring all build caches to be inactive.
    pub fn clear(
        directory: &Path,
        is_dry_run: bool,
    ) -> Result<ArtifactCacheRemoval, ArtifactCacheError> {
        if !Self::exists(directory)? {
            return Ok(ArtifactCacheRemoval::default());
        }

        // exclude all cache activity during validation and removal
        let _lock = Self::lock_directory(directory, ArtifactCacheLockMode::Exclusive)?;
        let builds_directory = directory.join("builds");

        // reject the operation before removing any active build
        if Self::exists(&builds_directory)? {
            for build in Self::read_directory(&builds_directory)? {
                let metadata = Self::metadata(&build)?;
                if !metadata.is_directory {
                    return Err(Self::invalid_entry(&build, "a build directory"));
                }

                if Self::is_build_active(&build)? {
                    return Err(ArtifactCacheError::BuildInUse { path: build });
                }
            }
        }

        // measure every record after validating the complete operation
        let (before_bytes, before_files) = Self::measure_records(directory)?;
        if is_dry_run {
            return Ok(ArtifactCacheRemoval {
                bytes: before_bytes,
                files: before_files,
            });
        }

        // retain the root lock while removing all cache records
        for entry in Self::read_directory(directory)? {
            if entry.file_name().is_some_and(|name| name == "lock") {
                continue;
            }

            Self::remove_path(&entry)?;
        }
        Ok(ArtifactCacheRemoval {
            bytes: before_bytes,
            files: before_files,
        })
    }

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

        // protect records written since the preceding collection
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
        let packs = Self::cached_files(&self.build_directory.join("packs"), "pack")?;
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

        // remove eligible inactive obsolete builds from oldest to newest
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

                if Self::is_build_active(&build.path)? {
                    continue;
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

    /// Read current manifests and require every selected pack.
    fn cached_manifests<R: DeserializeOwned>(
        &self,
        pack_files: &HashMap<&Path, (u64, SystemTime)>,
    ) -> Result<Vec<CachedManifest>, ArtifactCacheError> {
        let files = Self::cached_files(&self.build_directory.join("manifests"), "manifest")?;
        let mut manifests = Vec::with_capacity(files.len());
        for file in files {
            // decode and validate one current manifest
            let bytes = Self::read_file(&file.path)?;
            let manifest: ArtifactCacheManifest<R> = tspp_serde::from_slice(&bytes)
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
        directory: &Path,
        extension: &str,
    ) -> Result<Vec<CachedFile>, ArtifactCacheError> {
        // retain exact file sizes and modification times
        let mut files = Vec::new();
        for path in Self::read_directory(directory)? {
            let metadata = Self::metadata(&path)?;
            let has_extension = path.extension().is_some_and(|value| value == extension);
            if !metadata.is_file || !has_extension {
                return Err(Self::invalid_entry(&path, &format!("a .{extension} file")));
            }

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

    /// Measure every obsolete build without decoding its record format.
    fn obsolete_builds(&self) -> Result<Vec<CachedDirectory>, ArtifactCacheError> {
        let directory = self.directory.join("builds");

        // collect each non-current build as one opaque directory
        let mut builds = Vec::new();
        for path in Self::read_directory(&directory)? {
            if path != self.build_directory {
                builds.push(Self::cached_directory(path)?);
            }
        }

        Ok(builds)
    }

    /// Return whether one build lease is retained by another cache instance.
    fn is_build_active(build: &Path) -> Result<bool, ArtifactCacheError> {
        let path = build.join(Self::BUILD_LEASE_FILE);
        if !Self::exists(&path)? {
            return Ok(false);
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|error| ArtifactCacheError::FileSystem {
                operation: "open lease",
                path: path.clone(),
                message: error.to_string(),
            })?;

        // probe exclusive ownership without waiting for a live process
        match fs2::FileExt::try_lock_exclusive(&file) {
            Ok(()) => {
                fs2::FileExt::unlock(&file).map_err(|error| ArtifactCacheError::FileSystem {
                    operation: "unlock lease",
                    path,
                    message: error.to_string(),
                })?;

                Ok(false)
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => Ok(true),
            Err(error) => Err(ArtifactCacheError::FileSystem {
                operation: "lock lease",
                path,
                message: error.to_string(),
            }),
        }
    }

    /// Validate the exact current build directory entries.
    fn validate_current_build(&self) -> Result<(), ArtifactCacheError> {
        let expected = [
            self.build_directory.join(Self::BUILD_LEASE_FILE),
            self.build_directory.join("manifests"),
            self.build_directory.join("packs"),
        ];
        let entries = Self::read_directory(&self.build_directory)?;
        if entries != expected {
            return Err(ArtifactCacheError::Invalid(
                "current build must contain a lease, manifests, and packs".to_string(),
            )
            .record(&self.build_directory));
        }

        Ok(())
    }

    /// Measure one cache directory without decoding its records.
    fn cached_directory(path: PathBuf) -> Result<CachedDirectory, ArtifactCacheError> {
        let directory = Self::metadata(&path)?;
        if !directory.is_directory {
            return Err(Self::invalid_entry(&path, "a cache directory"));
        }

        let mut modified_at = directory.modified_at.ok_or_else(|| {
            ArtifactCacheError::Invalid("cache directory has no modification time".to_string())
                .record(&path)
        })?;
        let mut bytes = 0;
        let mut files = 0;
        let mut manifests = 0;
        let mut packs = 0;
        let mut pending = vec![path.to_path_buf()];

        // measure every descendant record exactly once
        while let Some(directory) = pending.pop() {
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

                    // count known record kinds
                    match entry.extension().and_then(|value| value.to_str()) {
                        Some("manifest") => manifests += 1,
                        Some("pack") => packs += 1,
                        _ => {}
                    }
                } else {
                    return Err(Self::invalid_entry(&entry, "a file or directory"));
                }
            }
        }

        Ok(CachedDirectory {
            path,
            bytes,
            files,
            manifests,
            packs,
            modified_at,
        })
    }

    /// Measure cache records while excluding the permanent root lock.
    fn measure_records(path: &Path) -> Result<(u64, u64), ArtifactCacheError> {
        let directory = Self::cached_directory(path.to_path_buf())?;
        let mut bytes = directory.bytes;
        let mut files = directory.files;
        let lock = path.join("lock");
        let metadata = Self::metadata(&lock)?;
        bytes -= metadata.size_bytes;
        files -= 1;

        Ok((bytes, files))
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
            return Err(Self::invalid_entry(&path, "a collection marker file"));
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

    /// Return one invalid cache entry error.
    fn invalid_entry(path: &Path, expected: &str) -> ArtifactCacheError {
        ArtifactCacheError::Invalid(format!("unexpected cache entry, expected {expected}"))
            .record(path)
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl ArtifactCache {
    /// Return empty cache usage where persistent storage is unavailable.
    pub fn measure(
        _directory: &Path,
        _build_id: BuildId,
    ) -> Result<ArtifactCacheUsage, ArtifactCacheError> {
        Ok(ArtifactCacheUsage::default())
    }

    /// Remove no records where persistent storage is unavailable.
    pub fn clear(
        _directory: &Path,
        _is_dry_run: bool,
    ) -> Result<ArtifactCacheRemoval, ArtifactCacheError> {
        Ok(ArtifactCacheRemoval::default())
    }

    /// Collect no records where persistent storage is unavailable.
    pub fn collect_if_due<R: DeserializeOwned>(
        &self,
        _retained_repository: &Path,
    ) -> Result<(), ArtifactCacheError> {
        Ok(())
    }
}

#[cfg(all(test, not(all(target_arch = "wasm32", target_os = "unknown"))))]
mod tests {
    use tspp_core::{BlobStore, StringPool};
    use tspp_source::TemporaryPhysicalFileSystem;

    use super::*;
    use crate::{ArtifactPack, ArtifactPackReference, ArtifactPackVersion};

    /// Retain a pack until its final repository selection is removed.
    #[test]
    fn test_collects_repository_selections() {
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

        // report the exact current build records
        let stats = ArtifactCache::measure(files.root(), BuildId::test()).expect("measure cache");
        assert_eq!(stats.current_build.builds, 1);
        assert_eq!(stats.current_build.manifests, 2);
        assert_eq!(stats.current_build.packs, 1);
        assert_eq!(stats.other_builds, ArtifactCacheStats::default());
        assert_eq!(stats.current_build, stats.total);

        // evict the first selection while retaining its shared pack
        let removed = cache
            .collect::<u64>(&retained_repository, SystemTime::now())
            .expect("collect repository selections");
        assert_eq!(removed.removed_files, 1);
        assert!(!cache.manifest_path(&first_repository).exists());
        assert!(cache.manifest_path(&retained_repository).exists());
        assert!(cache.pack_path(version).exists());
        assert!(
            cache
                .load::<u64>(&retained_repository, 1)
                .expect("load retained repository")
                .is_some()
        );

        // release the final selection and its pack
        let removed = cache
            .collect::<u64>(Path::new("unretained"), SystemTime::now())
            .expect("collect final repository selection");
        assert_eq!(removed.removed_files, 2);
        assert_eq!(removed.after_bytes, 0);
        assert!(!cache.manifest_path(&retained_repository).exists());
        assert!(!cache.pack_path(version).exists());
    }

    /// Preserve obsolete build caches until their final process exits.
    #[test]
    fn test_collects_inactive_builds() {
        let files = TemporaryPhysicalFileSystem::new_with_prefix("artifact-cache-build");
        let obsolete_build = BuildId::new([0xff; 16]);
        let obsolete = ArtifactCache::open(obsolete_build, files.root(), None)
            .expect("open obsolete artifact cache");
        let obsolete_record = PathBuf::from("builds")
            .join(obsolete_build.to_string())
            .join("opaque-record");
        files
            .write_bytes(&obsolete_record, b"obsolete")
            .expect("write obsolete build record");
        let current = ArtifactCache::open(BuildId::test(), files.root(), Some(0))
            .expect("open current artifact cache");

        // account for the active obsolete build in cache usage
        let stats = ArtifactCache::measure(files.root(), BuildId::test()).expect("measure cache");
        assert_eq!(stats.total.bytes, 8);
        assert_eq!(stats.total.builds, 2);
        assert_eq!(stats.current_build.bytes, 0);
        assert_eq!(stats.current_build.builds, 1);
        assert_eq!(stats.other_builds.bytes, 8);
        assert_eq!(stats.other_builds.builds, 1);

        // retain the obsolete build while its lease remains active
        let collected = current
            .collect::<u64>(Path::new("retained"), SystemTime::now())
            .expect("collect active build cache");
        assert_eq!(collected.before_bytes, 8);
        assert_eq!(collected.after_bytes, 8);
        assert_eq!(collected.removed_files, 0);
        assert!(files.root().join(&obsolete_record).exists());

        // remove the complete obsolete build after releasing its lease
        drop(obsolete);
        let collected = current
            .collect::<u64>(Path::new("retained"), SystemTime::now())
            .expect("collect inactive build cache");
        assert_eq!(collected.before_bytes, 8);
        assert_eq!(collected.after_bytes, 0);
        assert_eq!(collected.removed_files, 2);
        assert!(!files.root().join(obsolete_record).exists());
    }

    /// Preserve an active build while clearing the machine cache.
    #[test]
    fn test_clear_requires_inactive_builds() {
        let files = TemporaryPhysicalFileSystem::new_with_prefix("artifact-cache-clear");
        let cache =
            ArtifactCache::open(BuildId::test(), files.root(), None).expect("open artifact cache");

        // reject clearing while the build lease remains active
        let result = ArtifactCache::clear(files.root(), false);
        assert!(matches!(result, Err(ArtifactCacheError::BuildInUse { .. })));

        // preserve every cache record during a dry run
        drop(cache);
        let expected = ArtifactCache::clear(files.root(), true).expect("measure inactive cache");
        assert_eq!(expected, ArtifactCacheRemoval { bytes: 0, files: 1 });
        assert!(files.root().join("builds").exists());

        // remove every cache record after releasing the lease
        let removed = ArtifactCache::clear(files.root(), false).expect("clear inactive cache");
        assert_eq!(removed, expected);
        assert!(files.root().join("lock").exists());
        assert!(!files.root().join("builds").exists());
    }
}
