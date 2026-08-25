use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_source::FileSystem;
use serde::de::DeserializeOwned;

use crate::{
    ArtifactCacheError, ArtifactCacheManifest, ArtifactPack, ArtifactPackReference,
    ArtifactPackVersion, BuildId,
};

/// File extension for immutable artifact packs.
const PACK_EXTENSION: &str = "pack";
/// File extension for repository artifact manifests.
const MANIFEST_EXTENSION: &str = "manifest";
/// Stack buffer used to compare an existing immutable pack.
const PACK_COMPARE_BYTES: usize = 64 * 1024;

/// Persistent artifact packs selected by repository manifests.
#[derive(Debug)]
pub struct ArtifactCache {
    /// Destack build accepted by this cache instance.
    build_id: BuildId,
    /// Physical file system used for persistent cache state.
    files: Arc<dyn FileSystem>,
    /// Build-specific cache directory.
    directory: PathBuf,
}

impl ArtifactCache {
    /// Open one build-specific persistent artifact cache.
    pub fn open(
        build_id: BuildId,
        files: Arc<dyn FileSystem>,
        directory: impl Into<PathBuf>,
    ) -> Result<Self, ArtifactCacheError> {
        // FUGU #Incomplete: prune old builds and unreferenced packs against configured limits
        let directory = directory.into().join(build_id.to_string());
        Self::create_directory(files.as_ref(), &directory.join("packs"))?;
        Self::create_directory(files.as_ref(), &directory.join("manifests"))?;

        Ok(Self {
            build_id,
            files,
            directory,
        })
    }

    /// Return the Destack build accepted by this cache.
    pub fn build_id(&self) -> BuildId {
        self.build_id
    }

    /// Load one repository generation when present.
    pub fn load<R: DeserializeOwned>(
        &self,
        repository: &Path,
        worker_count: usize,
    ) -> Result<Option<(ArtifactCacheManifest<R>, Vec<ArtifactPack>)>, ArtifactCacheError> {
        let loaded = self.load_generation(repository, worker_count);

        self.discard_invalid(repository, loaded)
    }

    /// Load one repository manifest when present.
    pub fn manifest<R: DeserializeOwned>(
        &self,
        repository: &Path,
    ) -> Result<Option<ArtifactCacheManifest<R>>, ArtifactCacheError> {
        let loaded = self.read_manifest(repository);

        self.discard_invalid(repository, loaded)
    }

    /// Write one immutable artifact pack.
    pub fn write_pack(
        &self,
        version: ArtifactPackVersion,
        bytes: Vec<u8>,
    ) -> Result<u32, ArtifactCacheError> {
        let checksum = crc32fast::hash(&bytes);
        let path = self.pack_path(version);

        // retain an equal immutable pack already published by another writer
        if Self::exists(self.files.as_ref(), &path)? {
            if !Self::file_matches(self.files.as_ref(), &path, &bytes)? {
                return Err(ArtifactCacheError::Invalid(format!(
                    "pack {version} has conflicting bytes"
                )));
            }
        } else {
            Self::write_file(self.files.as_ref(), &path, &bytes)?;
        }

        Ok(checksum)
    }

    /// Atomically publish one repository manifest after its packs.
    pub fn write_manifest<R: serde::Serialize>(
        &self,
        repository: &Path,
        manifest: &ArtifactCacheManifest<R>,
    ) -> Result<(), ArtifactCacheError> {
        let bytes = destack_serde::to_vec(manifest)?;
        let path = self.manifest_path(repository);

        Self::write_file(self.files.as_ref(), &path, &bytes)
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
        let bytes = Self::read_file(self.files.as_ref(), &path)?;
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
        if !Self::exists(self.files.as_ref(), &path)? {
            return Ok(None);
        }
        let bytes = Self::read_file(self.files.as_ref(), &path)?;
        let manifest: ArtifactCacheManifest<R> = destack_serde::from_slice(&bytes)
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
            if !Self::exists(self.files.as_ref(), &self.pack_path(selected.version))? {
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
                    && Self::exists(self.files.as_ref(), &record)?
                {
                    Self::remove_file(self.files.as_ref(), &record)?;
                }
                if Self::exists(self.files.as_ref(), &manifest)? {
                    Self::remove_file(self.files.as_ref(), &manifest)?;
                }

                Err(error)
            }
            result => result,
        }
    }

    /// Resolve one versioned pack path.
    fn pack_path(&self, version: ArtifactPackVersion) -> PathBuf {
        self.directory
            .join("packs")
            .join(format!("{version}.{PACK_EXTENSION}"))
    }

    /// Resolve one repository manifest path.
    fn manifest_path(&self, repository: &Path) -> PathBuf {
        let repository = repository.to_string_lossy();
        let mut hasher = blake3::Hasher::new();
        hasher.update(&(repository.len() as u64).to_le_bytes());
        hasher.update(repository.as_bytes());
        let id = hasher.finalize().to_hex();

        self.directory
            .join("manifests")
            .join(format!("{id}.{MANIFEST_EXTENSION}"))
    }

    /// Create one cache directory recursively.
    fn create_directory(files: &dyn FileSystem, path: &Path) -> Result<(), ArtifactCacheError> {
        files
            .create_dir_all(path)
            .map_err(|error| ArtifactCacheError::FileSystem {
                operation: "create directory",
                path: path.to_path_buf(),
                message: error.to_string(),
            })
    }

    /// Return whether one cache path exists.
    fn exists(files: &dyn FileSystem, path: &Path) -> Result<bool, ArtifactCacheError> {
        files
            .exists(path)
            .map_err(|error| ArtifactCacheError::FileSystem {
                operation: "stat",
                path: path.to_path_buf(),
                message: error.to_string(),
            })
    }

    /// Read one complete cache file.
    fn read_file(files: &dyn FileSystem, path: &Path) -> Result<Vec<u8>, ArtifactCacheError> {
        files
            .read(path)
            .map_err(|error| ArtifactCacheError::FileSystem {
                operation: "read",
                path: path.to_path_buf(),
                message: error.to_string(),
            })
    }

    /// Compare one file with exact bytes without loading a second complete buffer.
    fn file_matches(
        files: &dyn FileSystem,
        path: &Path,
        bytes: &[u8],
    ) -> Result<bool, ArtifactCacheError> {
        let mut file = files
            .open(path)
            .map_err(|error| ArtifactCacheError::FileSystem {
                operation: "open",
                path: path.to_path_buf(),
                message: error.to_string(),
            })?;
        let mut buffer = [0; PACK_COMPARE_BYTES];
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
    fn write_file(
        files: &dyn FileSystem,
        path: &Path,
        bytes: &[u8],
    ) -> Result<(), ArtifactCacheError> {
        files
            .write(path, bytes)
            .map_err(|error| ArtifactCacheError::FileSystem {
                operation: "write",
                path: path.to_path_buf(),
                message: error.to_string(),
            })
    }

    /// Remove one exact cache file.
    fn remove_file(files: &dyn FileSystem, path: &Path) -> Result<(), ArtifactCacheError> {
        files
            .remove_file(path)
            .map_err(|error| ArtifactCacheError::FileSystem {
                operation: "remove",
                path: path.to_path_buf(),
                message: error.to_string(),
            })
    }
}
