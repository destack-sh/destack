use std::io::Read;
use std::path::{Path, PathBuf};

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::fs::{File, OpenOptions};

use serde::de::DeserializeOwned;
use tspp_source::{FileSystem, PhysicalFileSystem};

use crate::{
    ArtifactCacheError, ArtifactCacheManifest, ArtifactPack, ArtifactPackReference,
    ArtifactPackVersion, BuildId,
};

/// Persistent artifact packs selected by repository manifests.
#[derive(Debug)]
pub struct ArtifactCache {
    /// Toolchain build accepted by this cache instance.
    build_id: BuildId,
    /// Machine-local cache directory.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub(super) directory: PathBuf,
    /// Build-specific cache directory.
    pub(super) build_directory: PathBuf,
    /// Maximum retained cache size when bounded.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub(super) maximum_bytes: Option<u64>,
    /// Shared lease retained while this build cache is in use.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    _lease: ArtifactCacheLock,
}

/// Exclusive access to one artifact cache publication.
#[derive(Debug)]
pub struct ArtifactCachePublication<'a> {
    /// Cache receiving this publication.
    cache: &'a ArtifactCache,
    /// Cache lock retained for the publication lifetime.
    _lock: ArtifactCacheLock,
}

impl ArtifactCachePublication<'_> {
    /// Return the toolchain build producing this publication.
    pub fn build_id(&self) -> BuildId {
        self.cache.build_id()
    }

    /// Load one repository manifest when present.
    pub fn load_manifest<R: DeserializeOwned>(
        &self,
        repository: &Path,
    ) -> Result<Option<ArtifactCacheManifest<R>>, ArtifactCacheError> {
        let loaded = self.cache.read_manifest(repository);

        self.cache.discard_invalid(repository, loaded)
    }

    /// Write one immutable artifact pack.
    pub fn write_pack(
        &self,
        version: ArtifactPackVersion,
        bytes: Vec<u8>,
    ) -> Result<u32, ArtifactCacheError> {
        self.cache.write_pack(version, bytes)
    }

    /// Atomically publish one repository manifest after its packs.
    pub fn write_manifest<R: serde::Serialize>(
        &self,
        repository: &Path,
        manifest: &ArtifactCacheManifest<R>,
    ) -> Result<(), ArtifactCacheError> {
        self.cache.write_manifest(repository, manifest)
    }
}

/// Operating-system lock retained around cache reads or mutation.
#[derive(Debug)]
pub(super) struct ArtifactCacheLock {
    /// Locked cache file on native hosts.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    _file: File,
}

/// Access mode held by one artifact cache lock.
#[derive(Debug, Clone, Copy)]
pub(super) enum ArtifactCacheLockMode {
    /// Concurrent cache restoration.
    Shared,
    /// Exclusive cache mutation.
    Exclusive,
}

impl ArtifactCache {
    /// File extension for immutable artifact packs.
    const PACK_EXTENSION: &'static str = "pack";
    /// File extension for repository artifact manifests.
    const MANIFEST_EXTENSION: &'static str = "manifest";
    /// File retaining one build cache while a process uses it.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub(super) const BUILD_LEASE_FILE: &'static str = "lease";
    /// Stack buffer used to compare an existing immutable pack.
    const PACK_COMPARE_BYTES: usize = 64 * 1024;

    /// Open one build-specific persistent artifact cache.
    pub fn open(
        build_id: BuildId,
        directory: impl Into<PathBuf>,
        maximum_bytes: Option<u64>,
    ) -> Result<Self, ArtifactCacheError> {
        let directory = directory.into();
        let build_directory = directory.join("builds").join(build_id.to_string());

        // exclude machine cache clearing while opening this build
        let _lock = Self::lock_directory(&directory, ArtifactCacheLockMode::Exclusive)?;
        Self::create_directory(&build_directory.join("packs"))?;
        Self::create_directory(&build_directory.join("manifests"))?;

        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        let lease = Self::lock_path(
            &build_directory.join(Self::BUILD_LEASE_FILE),
            ArtifactCacheLockMode::Shared,
        )?;

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        let _ = maximum_bytes;

        Ok(Self {
            build_id,
            #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
            directory,
            build_directory,
            #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
            maximum_bytes,
            #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
            _lease: lease,
        })
    }

    /// Return the Toolchain build accepted by this cache.
    pub fn build_id(&self) -> BuildId {
        self.build_id
    }

    /// Begin one exclusive artifact cache publication.
    pub fn publish(&self) -> Result<ArtifactCachePublication<'_>, ArtifactCacheError> {
        let lock = self.lock(ArtifactCacheLockMode::Exclusive)?;

        Ok(ArtifactCachePublication {
            cache: self,
            _lock: lock,
        })
    }

    /// Load one repository generation when present.
    pub fn load<R: DeserializeOwned>(
        &self,
        repository: &Path,
        worker_count: usize,
    ) -> Result<Option<(ArtifactCacheManifest<R>, Vec<ArtifactPack>)>, ArtifactCacheError> {
        let lock = self.lock(ArtifactCacheLockMode::Shared)?;
        let loaded = self.load_generation(repository, worker_count);

        // revalidate invalid records under exclusive access before removing them
        if loaded
            .as_ref()
            .is_err_and(ArtifactCacheError::is_invalid_record)
        {
            drop(lock);
            let _lock = self.lock(ArtifactCacheLockMode::Exclusive)?;
            let loaded = self.load_generation(repository, worker_count);
            let loaded = self.discard_invalid(repository, loaded)?;
            if loaded.is_some() {
                self.touch_manifest(repository)?;
            }

            return Ok(loaded);
        }

        // update repository recency after one successful restoration
        let loaded = loaded?;
        if loaded.is_some() {
            self.touch_manifest(repository)?;
        }

        Ok(loaded)
    }

    /// Write one immutable artifact pack.
    fn write_pack(
        &self,
        version: ArtifactPackVersion,
        bytes: Vec<u8>,
    ) -> Result<u32, ArtifactCacheError> {
        let checksum = crc32fast::hash(&bytes);
        let path = self.pack_path(version);

        // retain an equal immutable pack already published by another writer
        if Self::exists(&path)? {
            if !Self::file_matches(&path, &bytes)? {
                return Err(ArtifactCacheError::Invalid(format!(
                    "pack {version} has conflicting bytes"
                )));
            }
        } else {
            Self::write_file(&path, &bytes)?;
        }

        Ok(checksum)
    }

    /// Atomically publish one repository manifest after its packs.
    fn write_manifest<R: serde::Serialize>(
        &self,
        repository: &Path,
        manifest: &ArtifactCacheManifest<R>,
    ) -> Result<(), ArtifactCacheError> {
        let bytes = tspp_serde::to_vec(manifest)?;
        let path = self.manifest_path(repository);

        Self::write_file(&path, &bytes)
    }

    /// Load and validate one repository manifest and every selected pack.
    fn load_generation<R: DeserializeOwned>(
        &self,
        repository: &Path,
        worker_count: usize,
    ) -> Result<Option<(ArtifactCacheManifest<R>, Vec<ArtifactPack>)>, ArtifactCacheError> {
        let Some(manifest) = self.read_manifest(repository)? else {
            return Ok(None);
        };
        let worker_count = worker_count.max(1).min(manifest.packs.len().max(1));

        // load independent immutable packs across bounded workers
        let loaded = if worker_count == 1 {
            manifest
                .packs
                .iter()
                .map(|selected| self.load_pack(*selected))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            std::thread::scope(|scope| -> Result<Vec<ArtifactPack>, ArtifactCacheError> {
                let selected = &manifest.packs;
                let mut handles = Vec::with_capacity(worker_count);
                for worker in 0..worker_count {
                    handles.push(scope.spawn(move || {
                        selected
                            .iter()
                            .copied()
                            .skip(worker)
                            .step_by(worker_count)
                            .map(|selected| self.load_pack(selected))
                            .collect::<Result<Vec<_>, _>>()
                    }));
                }

                let mut loaded = Vec::with_capacity(manifest.packs.len());
                for handle in handles {
                    let packs = handle.join().map_err(|_| {
                        ArtifactCacheError::Internal("artifact cache loader panicked".to_string())
                    })??;
                    loaded.extend(packs);
                }

                Ok(loaded)
            })?
        };

        Ok(Some((manifest, loaded)))
    }

    /// Load and validate one immutable pack selected by a manifest.
    fn load_pack(
        &self,
        selected: ArtifactPackReference,
    ) -> Result<ArtifactPack, ArtifactCacheError> {
        let version = selected.version;
        let path = self.pack_path(version);
        let bytes = Self::read_file(&path)?;
        let checksum = crc32fast::hash(&bytes);
        if checksum != selected.checksum {
            let error =
                ArtifactCacheError::Invalid(format!("pack {version} has an invalid checksum"));

            return Err(error.record(&path));
        }

        // decode and verify the selected content identity
        let pack =
            ArtifactPack::decode(bytes, self.build_id).map_err(|error| error.record(&path))?;
        if pack.package() != selected.package {
            let error = ArtifactCacheError::Invalid(format!(
                "pack {version} belongs to package {:?}, expected {:?}",
                pack.package(),
                selected.package
            ));

            return Err(error.record(&path));
        }
        let stored_version = ArtifactPackVersion::new(pack.versions());
        if stored_version != selected.version {
            let error = ArtifactCacheError::Invalid(format!(
                "pack {version} contains version {stored_version:?}, expected {:?}",
                selected.version
            ));

            return Err(error.record(&path));
        }

        Ok(pack)
    }

    /// Read and validate one repository manifest.
    fn read_manifest<R: DeserializeOwned>(
        &self,
        repository: &Path,
    ) -> Result<Option<ArtifactCacheManifest<R>>, ArtifactCacheError> {
        let path = self.manifest_path(repository);
        if !Self::exists(&path)? {
            return Ok(None);
        }
        let bytes = Self::read_file(&path)?;
        let manifest: ArtifactCacheManifest<R> = tspp_serde::from_slice(&bytes)
            .map_err(ArtifactCacheError::from)
            .map_err(|error| error.record(&path))?;
        manifest
            .validate(self.build_id)
            .map_err(|error| error.record(&path))?;
        if !manifest.belongs_to(repository) {
            let error =
                ArtifactCacheError::Invalid("manifest belongs to another repository".to_string());

            return Err(error.record(&path));
        }

        // require every immutable pack selected by this manifest
        for selected in &manifest.packs {
            if !Self::exists(&self.pack_path(selected.version))? {
                let error = ArtifactCacheError::Invalid(format!(
                    "manifest references missing pack {}",
                    selected.version
                ));

                return Err(error.record(&path));
            }
        }

        Ok(Some(manifest))
    }

    /// Remove an invalid cache record before returning its error.
    fn discard_invalid<T>(
        &self,
        repository: &Path,
        loaded: Result<Option<T>, ArtifactCacheError>,
    ) -> Result<Option<T>, ArtifactCacheError> {
        match loaded {
            Err(error) if error.is_invalid_record() => {
                // remove the invalid record and this repository manifest
                let manifest = self.manifest_path(repository);
                let record = error.record_path().map(Path::to_path_buf);
                if let Some(record) = record
                    && record != manifest
                    && Self::exists(&record)?
                {
                    Self::remove_file(&record)?;
                }
                if Self::exists(&manifest)? {
                    Self::remove_file(&manifest)?;
                }

                Err(error)
            }
            result => result,
        }
    }

    /// Resolve one versioned pack path.
    pub(super) fn pack_path(&self, version: ArtifactPackVersion) -> PathBuf {
        self.build_directory
            .join("packs")
            .join(format!("{version}.{}", Self::PACK_EXTENSION))
    }

    /// Resolve one repository manifest path.
    pub(super) fn manifest_path(&self, repository: &Path) -> PathBuf {
        let repository = repository.to_string_lossy();
        let mut hasher = blake3::Hasher::new();
        hasher.update(&(repository.len() as u64).to_le_bytes());
        hasher.update(repository.as_bytes());
        let id = hasher.finalize().to_hex();

        self.build_directory
            .join("manifests")
            .join(format!("{id}.{}", Self::MANIFEST_EXTENSION))
    }

    /// Lock this cache for shared reads or exclusive mutation.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub(super) fn lock(
        &self,
        mode: ArtifactCacheLockMode,
    ) -> Result<ArtifactCacheLock, ArtifactCacheError> {
        Self::lock_directory(&self.directory, mode)
    }

    /// Lock one machine cache directory.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub(super) fn lock_directory(
        directory: &Path,
        mode: ArtifactCacheLockMode,
    ) -> Result<ArtifactCacheLock, ArtifactCacheError> {
        Self::create_directory(directory)?;
        Self::lock_path(&directory.join("lock"), mode)
    }

    /// Lock one exact cache file.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub(super) fn lock_path(
        path: &Path,
        mode: ArtifactCacheLockMode,
    ) -> Result<ArtifactCacheLock, ArtifactCacheError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|error| ArtifactCacheError::FileSystem {
                operation: "open lock",
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;

        // coordinate every process sharing this machine cache
        let result = match mode {
            ArtifactCacheLockMode::Shared => fs2::FileExt::lock_shared(&file),
            ArtifactCacheLockMode::Exclusive => fs2::FileExt::lock_exclusive(&file),
        };
        result.map_err(|error| ArtifactCacheError::FileSystem {
            operation: "lock",
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

        Ok(ArtifactCacheLock { _file: file })
    }

    /// Return an inert cache lock where persistent storage is unavailable.
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    pub(super) fn lock(
        &self,
        _mode: ArtifactCacheLockMode,
    ) -> Result<ArtifactCacheLock, ArtifactCacheError> {
        Ok(ArtifactCacheLock {})
    }

    /// Return an inert directory lock where persistent storage is unavailable.
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    pub(super) fn lock_directory(
        _directory: &Path,
        _mode: ArtifactCacheLockMode,
    ) -> Result<ArtifactCacheLock, ArtifactCacheError> {
        Ok(ArtifactCacheLock {})
    }

    /// Record one successful repository cache restoration.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    fn touch_manifest(&self, repository: &Path) -> Result<(), ArtifactCacheError> {
        let path = self.manifest_path(repository);
        let file = OpenOptions::new()
            .write(true)
            .open(&path)
            .map_err(|error| ArtifactCacheError::FileSystem {
                operation: "open manifest",
                path: path.clone(),
                message: error.to_string(),
            })?;
        let times = std::fs::FileTimes::new().set_modified(std::time::SystemTime::now());
        file.set_times(times)
            .map_err(|error| ArtifactCacheError::FileSystem {
                operation: "touch manifest",
                path,
                message: error.to_string(),
            })
    }

    /// Ignore cache access time where persistent storage is unavailable.
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    fn touch_manifest(&self, _repository: &Path) -> Result<(), ArtifactCacheError> {
        Ok(())
    }

    /// Create one cache directory recursively.
    pub(super) fn create_directory(path: &Path) -> Result<(), ArtifactCacheError> {
        PhysicalFileSystem
            .create_dir_all(path)
            .map_err(|error| ArtifactCacheError::FileSystem {
                operation: "create directory",
                path: path.to_path_buf(),
                message: error.to_string(),
            })
    }

    /// Return whether one cache path exists.
    pub(super) fn exists(path: &Path) -> Result<bool, ArtifactCacheError> {
        PhysicalFileSystem
            .exists(path)
            .map_err(|error| ArtifactCacheError::FileSystem {
                operation: "stat",
                path: path.to_path_buf(),
                message: error.to_string(),
            })
    }

    /// Read one complete cache file.
    pub(super) fn read_file(path: &Path) -> Result<Vec<u8>, ArtifactCacheError> {
        PhysicalFileSystem
            .read(path)
            .map_err(|error| ArtifactCacheError::FileSystem {
                operation: "read",
                path: path.to_path_buf(),
                message: error.to_string(),
            })
    }

    /// Compare one file with exact bytes without loading a second complete buffer.
    fn file_matches(path: &Path, bytes: &[u8]) -> Result<bool, ArtifactCacheError> {
        let mut file =
            PhysicalFileSystem
                .open(path)
                .map_err(|error| ArtifactCacheError::FileSystem {
                    operation: "open",
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })?;
        let mut buffer = [0; Self::PACK_COMPARE_BYTES];
        let mut offset = 0;

        // compare each physical chunk with the expected pack bytes
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|error| ArtifactCacheError::FileSystem {
                    operation: "read",
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })?;
            if read == 0 {
                return Ok(offset == bytes.len());
            }
            let Some(end) = offset.checked_add(read) else {
                return Ok(false);
            };
            if bytes.get(offset..end) != Some(&buffer[..read]) {
                return Ok(false);
            }
            offset = end;
        }
    }

    /// Atomically write one complete cache file.
    pub(super) fn write_file(path: &Path, bytes: &[u8]) -> Result<(), ArtifactCacheError> {
        PhysicalFileSystem
            .write(path, bytes)
            .map_err(|error| ArtifactCacheError::FileSystem {
                operation: "write",
                path: path.to_path_buf(),
                message: error.to_string(),
            })
    }

    /// Remove one exact cache file.
    pub(super) fn remove_file(path: &Path) -> Result<(), ArtifactCacheError> {
        PhysicalFileSystem
            .remove_file(path)
            .map_err(|error| ArtifactCacheError::FileSystem {
                operation: "remove",
                path: path.to_path_buf(),
                message: error.to_string(),
            })
    }
}
