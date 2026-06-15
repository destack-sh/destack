use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

use destack_core::StringPool;

use crate::store::ArtifactCache;
use crate::{
    ArtifactBlobError, ArtifactRecord, ArtifactVersion, CACHE_BLOB_LIMIT_BYTES, CacheStore,
    CacheStoreError, RepositoryCacheLayout,
};

/// Persistent store for exact artifact records.
pub trait ArtifactStore: std::fmt::Debug + Send + Sync {
    /// Load one exact artifact record when it is present.
    fn load(&self, expected: &ArtifactVersion)
    -> Result<Option<ArtifactRecord>, ArtifactBlobError>;

    /// Store one exact artifact record when this store persists artifacts.
    fn store(
        &self,
        version: &ArtifactVersion,
        cache: &ArtifactCache,
        strings: &StringPool,
    ) -> Result<(), ArtifactBlobError>;

    /// Retain only reachable artifact records.
    fn retain(&self, reachable: &HashSet<ArtifactVersion>) -> Result<(), ArtifactBlobError>;
}

/// Artifact store that disables persistence.
#[derive(Debug, Default, Clone)]
pub struct NullArtifactStore;

impl NullArtifactStore {
    /// Create a null artifact store.
    pub fn new() -> Self {
        Self
    }
}

impl ArtifactStore for NullArtifactStore {
    fn load(
        &self,
        _expected: &ArtifactVersion,
    ) -> Result<Option<ArtifactRecord>, ArtifactBlobError> {
        Ok(None)
    }

    fn store(
        &self,
        _version: &ArtifactVersion,
        _cache: &ArtifactCache,
        _strings: &StringPool,
    ) -> Result<(), ArtifactBlobError> {
        Ok(())
    }

    fn retain(&self, _reachable: &HashSet<ArtifactVersion>) -> Result<(), ArtifactBlobError> {
        Ok(())
    }
}

/// Disk-backed store of canonical artifact records.
#[derive(Debug)]
pub struct DiskArtifactStore {
    /// The byte store backing this artifact store.
    store: Arc<dyn CacheStore>,
    /// The repository cache layout.
    layout: RepositoryCacheLayout,
    /// The build fingerprint partition this artifact store serves.
    build_fingerprint: String,
}

impl DiskArtifactStore {
    /// Create one disk artifact store.
    pub fn new(
        store: Arc<dyn CacheStore>,
        layout: RepositoryCacheLayout,
        build_fingerprint: impl Into<String>,
    ) -> Self {
        Self {
            store,
            layout,
            build_fingerprint: build_fingerprint.into(),
        }
    }

    /// Load one exact artifact record.
    fn load_record(
        &self,
        expected: &ArtifactVersion,
    ) -> Result<Option<ArtifactRecord>, ArtifactBlobError> {
        // read cached bytes
        let Some(bytes) = self.read_record_bytes(expected)? else {
            return Ok(None);
        };

        // decode and validate record
        let record = postcard::from_bytes::<ArtifactRecord>(&bytes)
            .map_err(|error| ArtifactBlobError::Codec(Box::new(error)))?;
        if record.version != *expected {
            return Err(ArtifactBlobError::Version {
                expected: Box::new(*expected),
                found: Box::new(record.version),
            });
        }

        Ok(Some(record))
    }

    /// Store one exact artifact record.
    fn store_record(&self, record: &ArtifactRecord) -> Result<(), ArtifactBlobError> {
        // encode record
        let bytes = postcard::to_allocvec(record)
            .map_err(|error| ArtifactBlobError::Codec(Box::new(error)))?;
        let byte_len = bytes.len() as u64;
        if byte_len > CACHE_BLOB_LIMIT_BYTES {
            return Err(ArtifactBlobError::Size {
                limit: CACHE_BLOB_LIMIT_BYTES,
                actual: byte_len,
            });
        }

        // write under cache lock
        self.with_write_lock(|| {
            self.write_record_bytes(&record.version, &bytes)?;

            Ok(())
        })?;

        Ok(())
    }

    /// Retain only reachable artifact records.
    fn retain_reachable(
        &self,
        reachable: &HashSet<ArtifactVersion>,
    ) -> Result<(), ArtifactBlobError> {
        // build retained path set
        let root = self.layout.artifact_root(&self.build_fingerprint);
        let retained_paths = reachable
            .iter()
            .map(|version| self.record_path(version))
            .collect::<HashSet<_>>();

        // remove unreachable entries
        self.with_write_lock(|| {
            for path in self.store.entries(&root)? {
                if !retained_paths.contains(&path) {
                    self.store.remove(&path)?;
                }
            }

            Ok(())
        })
    }

    /// Run one write operation under the persistent cache lock.
    fn with_write_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, ArtifactBlobError>,
    ) -> Result<T, ArtifactBlobError> {
        // acquire cache lock
        let lock_path = self.layout.cache_lock_path();
        let mut operation = Some(operation);
        let mut output = None;

        // run operation once
        self.store.with_exclusive_lock(&lock_path, &mut || {
            let Some(operation) = operation.take() else {
                panic!("cache lock should run exactly once");
            };
            let result = operation();
            output = Some(result);
        })?;

        // return operation result
        match output {
            Some(output) => output,
            None => panic!("cache lock should produce one value"),
        }
    }

    /// Read one exact record.
    fn read_record_bytes(
        &self,
        expected: &ArtifactVersion,
    ) -> Result<Option<Vec<u8>>, ArtifactBlobError> {
        // inspect cached record
        let record_path = self.record_path(expected);
        let Some(byte_len) = self.store.byte_len(&record_path)? else {
            return Ok(None);
        };
        if byte_len > CACHE_BLOB_LIMIT_BYTES {
            return Err(ArtifactBlobError::Size {
                limit: CACHE_BLOB_LIMIT_BYTES,
                actual: byte_len,
            });
        }

        // read cached bytes
        let Some(bytes) = self.store.read(&record_path)? else {
            return Ok(None);
        };

        Ok(Some(bytes))
    }

    /// Write one exact record blob.
    fn write_record_bytes(
        &self,
        version: &ArtifactVersion,
        bytes: &[u8],
    ) -> Result<(), ArtifactBlobError> {
        // accept existing identical bytes
        let record_path = self.record_path(version);
        if let Some(existing_bytes) = self.store.read(&record_path)? {
            if existing_bytes != bytes {
                return Err(ArtifactBlobError::Conflict {
                    version: Box::new(*version),
                });
            }

            return Ok(());
        }

        // write new bytes
        match self.store.write_once(&record_path, bytes) {
            Ok(()) => {}
            Err(CacheStoreError::AlreadyExists) => {
                let Some(existing_bytes) = self.store.read(&record_path)? else {
                    return Err(ArtifactBlobError::Corrupt(
                        "cache entry disappeared after write conflict",
                    ));
                };
                if existing_bytes != bytes {
                    return Err(ArtifactBlobError::Conflict {
                        version: Box::new(*version),
                    });
                }
            }
            Err(error) => return Err(error.into()),
        }

        Ok(())
    }

    /// Return the cached record path for one exact version.
    fn record_path(&self, version: &ArtifactVersion) -> PathBuf {
        let fingerprint = version.fingerprint.to_string();
        let shard = &fingerprint[1..3];

        self.layout
            .artifact_root(&self.build_fingerprint)
            .join(shard)
            .join(format!("{fingerprint}.bin"))
    }
}

impl From<CacheStoreError> for ArtifactBlobError {
    fn from(error: CacheStoreError) -> Self {
        Self::Store(Box::new(error))
    }
}

impl ArtifactStore for DiskArtifactStore {
    fn load(
        &self,
        expected: &ArtifactVersion,
    ) -> Result<Option<ArtifactRecord>, ArtifactBlobError> {
        self.load_record(expected)
    }

    fn store(
        &self,
        version: &ArtifactVersion,
        cache: &ArtifactCache,
        strings: &StringPool,
    ) -> Result<(), ArtifactBlobError> {
        let Some(record) = cache.record(version, strings)? else {
            return Err(ArtifactBlobError::Corrupt(
                "artifact store cannot persist missing artifact version",
            ));
        };

        self.store_record(&record)
    }

    fn retain(&self, reachable: &HashSet<ArtifactVersion>) -> Result<(), ArtifactBlobError> {
        self.retain_reachable(reachable)
    }
}
