use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_core::{StringId, StringPool, stable_hash_bytes_128};
use parking_lot::{Mutex, RwLock};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{
    ARTIFACT_SEGMENT_EXTENSION, ARTIFACT_STORE_VERSION, ArtifactError, ArtifactFlush,
    ArtifactRecord, ArtifactStore, ArtifactVersion, BlobStore, BlobStoreError, MAX_BLOB_BYTES,
    RepositoryStoreLayout,
};

/// Segmented store of artifacts.
#[derive(Debug)]
pub struct SegmentedArtifactStore {
    /// The byte store backing this artifact store.
    store: Arc<dyn BlobStore>,
    /// The repository store layout.
    layout: RepositoryStoreLayout,
    /// The persistent store partition for this build and record format.
    partition: String,
    /// Artifacts waiting for publication.
    pending: Mutex<Vec<ArtifactRecord>>,
    /// Loaded artifacts keyed by exact version.
    records: RwLock<FxHashMap<ArtifactVersion, ArtifactRecord>>,
    /// Segment files already reflected in the process-local index.
    segments: RwLock<HashSet<PathBuf>>,
    /// Whether the persistent segment directory has been indexed.
    is_index_loaded: RwLock<bool>,
}

impl SegmentedArtifactStore {
    /// Create one segment artifact store.
    pub fn new(
        store: Arc<dyn BlobStore>,
        layout: RepositoryStoreLayout,
        build_fingerprint: impl Into<String>,
    ) -> Self {
        let build_fingerprint = build_fingerprint.into();
        let partition = format!("{build_fingerprint}-artifact-{ARTIFACT_STORE_VERSION}");

        Self {
            store,
            layout,
            partition,
            pending: Mutex::new(Vec::new()),
            records: RwLock::new(FxHashMap::default()),
            segments: RwLock::new(HashSet::new()),
            is_index_loaded: RwLock::new(false),
        }
    }

    /// Load one artifact by exact version.
    fn load_record(
        &self,
        version: &ArtifactVersion,
        strings: &StringPool,
    ) -> Result<Option<ArtifactRecord>, ArtifactError> {
        // check artifacts written or loaded by this process first
        if let Some(record) = self.load_from_index(version) {
            return Ok(Some(record));
        }

        // load persistent segment index on first process-local miss
        self.load_index(strings)?;

        Ok(self.load_from_index(version))
    }

    /// Load one artifact from the process-local index.
    fn load_from_index(&self, version: &ArtifactVersion) -> Option<ArtifactRecord> {
        let records = self.records.read();
        let record = records.get(version)?;

        Some(record.clone())
    }

    /// Flush queued artifacts as one immutable segment.
    fn flush_pending(&self, strings: &StringPool) -> Result<ArtifactFlush, ArtifactError> {
        // drain queued records
        let records = {
            let mut pending = self.pending.lock();
            if pending.is_empty() {
                return Ok(ArtifactFlush::default());
            }

            std::mem::take(&mut *pending)
        };

        // refresh and publish under the cross-process store lock
        let flush = self.with_write_lock(|| {
            self.load_new_segments(strings)?;
            let known_versions = self.records.read().keys().copied().collect::<HashSet<_>>();
            let segment =
                ArtifactSegment::build(&self.partition, strings, &records, &known_versions)?;
            if segment.records.is_empty() {
                return Ok(ArtifactFlush::default());
            }

            self.write_segment(segment, strings)
        });
        if let Err(error) = flush {
            self.pending.lock().extend(records);

            Err(error)
        } else {
            flush
        }
    }

    /// Write one immutable segment.
    fn write_segment(
        &self,
        segment: ArtifactSegment,
        strings: &StringPool,
    ) -> Result<ArtifactFlush, ArtifactError> {
        // encode and name the segment
        let bytes = segment.encode()?;
        let path = self.segment_path(&bytes);
        let flush = ArtifactFlush {
            segments: 1,
            artifacts: segment.records.len(),
            strings: segment.strings.len(),
            bytes: bytes.len(),
        };

        // publish the segment and update any loaded index
        self.write_segment_bytes(&path, &bytes)?;
        self.index_segment(segment, strings)?;
        self.segments.write().insert(path);

        Ok(flush)
    }

    /// Write one immutable segment.
    fn write_segment_bytes(&self, path: &Path, bytes: &[u8]) -> Result<(), ArtifactError> {
        // accept existing identical bytes
        if let Some(existing_bytes) = self.store.read(path)? {
            if existing_bytes != bytes {
                return Err(ArtifactError::Invalid("artifact segment hash collision"));
            }

            return Ok(());
        }

        // write new segment bytes
        match self.store.write_once(path, bytes) {
            Ok(()) => {}
            Err(BlobStoreError::AlreadyExists) => {
                let Some(existing_bytes) = self.store.read(path)? else {
                    return Err(ArtifactError::Invalid(
                        "artifact segment disappeared after write conflict",
                    ));
                };
                if existing_bytes != bytes {
                    return Err(ArtifactError::Invalid("artifact segment hash collision"));
                }
            }
            Err(error) => return Err(error.into()),
        }

        Ok(())
    }

    /// Retain only selected artifact versions.
    fn retain_versions(
        &self,
        versions: &HashSet<ArtifactVersion>,
        strings: &StringPool,
    ) -> Result<(), ArtifactError> {
        // discard queued records that are no longer selected
        self.pending
            .lock()
            .retain(|record| versions.contains(&record.version));
        let _flush = self.flush_pending(strings)?;

        // compact artifact records under the store lock
        self.with_write_lock(|| {
            self.load_new_segments(strings)?;
            let mut retained_records = FxHashMap::default();
            let mut retained_segments = HashSet::new();
            let mut stored_versions = HashSet::new();

            for path in self.segment_paths()? {
                let Some(segment) = self.read_segment(&path)? else {
                    continue;
                };
                let records = segment
                    .records
                    .iter()
                    .filter(|record| {
                        versions.contains(&record.version) && stored_versions.insert(record.version)
                    })
                    .cloned()
                    .collect::<Vec<_>>();

                // drop fully unreachable segments
                if records.is_empty() {
                    self.store.remove(&path)?;
                }
                // keep fully reachable segments
                else if records.len() == segment.records.len() {
                    segment.insert_into(strings, &mut retained_records)?;
                    retained_segments.insert(path);

                    continue;
                }
                // compact partially reachable segments
                else {
                    let segment = segment.compact(records);
                    let bytes = segment.encode()?;
                    let compacted_path = self.segment_path(&bytes);

                    self.write_segment_bytes(&compacted_path, &bytes)?;
                    if compacted_path != path {
                        self.store.remove(&path)?;
                    }
                    segment.insert_into(strings, &mut retained_records)?;
                    retained_segments.insert(compacted_path);
                }
            }

            *self.records.write() = retained_records;
            *self.segments.write() = retained_segments;
            *self.is_index_loaded.write() = true;

            Ok(())
        })?;

        Ok(())
    }

    /// Ensure the in-memory segment index is populated.
    fn load_index(&self, strings: &StringPool) -> Result<(), ArtifactError> {
        if *self.is_index_loaded.read() {
            return Ok(());
        }

        let mut is_index_loaded = self.is_index_loaded.write();
        if *is_index_loaded {
            return Ok(());
        }

        self.load_new_segments(strings)?;
        *is_index_loaded = true;

        Ok(())
    }

    /// Add every newly published segment to the process-local indexes.
    fn load_new_segments(&self, strings: &StringPool) -> Result<(), ArtifactError> {
        let known_segments = self.segments.read();
        let paths = self
            .segment_paths()?
            .into_iter()
            .filter(|path| !known_segments.contains(path))
            .collect::<Vec<_>>();
        drop(known_segments);

        let mut records = self.records.write();
        let mut known_segments = self.segments.write();
        for path in paths {
            let Some(segment) = self.read_segment(&path)? else {
                continue;
            };

            segment.insert_into(strings, &mut records)?;
            known_segments.insert(path);
        }

        Ok(())
    }

    /// Add one segment to the process-local index.
    fn index_segment(
        &self,
        segment: ArtifactSegment,
        strings: &StringPool,
    ) -> Result<(), ArtifactError> {
        let mut records = self.records.write();

        segment.insert_into(strings, &mut records)
    }

    /// Read one segment file from disk.
    fn read_segment(&self, path: &Path) -> Result<Option<ArtifactSegment>, ArtifactError> {
        // inspect segment size
        let Some(byte_len) = self.store.byte_len(path)? else {
            return Ok(None);
        };
        if byte_len > MAX_BLOB_BYTES {
            return Err(ArtifactError::Size {
                limit: MAX_BLOB_BYTES,
                actual: byte_len,
            });
        }

        // decode segment bytes
        let Some(bytes) = self.store.read(path)? else {
            return Ok(None);
        };
        let segment = ArtifactSegment::decode(&bytes)?;
        if segment.partition != self.partition {
            return Err(ArtifactError::Invalid(
                "artifact segment does not match its store partition",
            ));
        }

        Ok(Some(segment))
    }

    /// Return all artifact segment paths in this build partition.
    fn segment_paths(&self) -> Result<Vec<PathBuf>, ArtifactError> {
        let root = self.layout.artifact_root(&self.partition);
        let mut paths = self
            .store
            .entries(&root)?
            .into_iter()
            .filter(|path| {
                path.extension().and_then(|extension| extension.to_str())
                    == Some(ARTIFACT_SEGMENT_EXTENSION)
            })
            .collect::<Vec<_>>();
        paths.sort_unstable();

        Ok(paths)
    }

    /// Return the stored segment path for encoded segment bytes.
    fn segment_path(&self, bytes: &[u8]) -> PathBuf {
        let fingerprint = format!("{:032x}", stable_hash_bytes_128(bytes));
        let shard = &fingerprint[0..2];

        self.layout
            .artifact_root(&self.partition)
            .join(shard)
            .join(format!("{fingerprint}.{ARTIFACT_SEGMENT_EXTENSION}"))
    }

    /// Run one write operation under the persistent store lock.
    fn with_write_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, ArtifactError>,
    ) -> Result<T, ArtifactError> {
        // acquire store lock
        let lock_path = self.layout.store_lock_path();
        let mut operation = Some(operation);
        let mut output = None;

        // run operation once
        self.store.with_exclusive_lock(&lock_path, &mut || {
            let Some(operation) = operation.take() else {
                unreachable!("blob store lock should run the operation exactly once");
            };
            let result = operation();
            output = Some(result);
        })?;

        // return operation result
        match output {
            Some(output) => output,
            None => unreachable!("blob store lock should run the operation before returning"),
        }
    }
}

impl ArtifactStore for SegmentedArtifactStore {
    fn load(
        &self,
        version: &ArtifactVersion,
        strings: &StringPool,
    ) -> Result<Option<ArtifactRecord>, ArtifactError> {
        self.load_record(version, strings)
    }

    fn store(&self, record: ArtifactRecord) -> Result<(), ArtifactError> {
        self.pending.lock().push(record);

        Ok(())
    }

    fn flush(&self, strings: &StringPool) -> Result<ArtifactFlush, ArtifactError> {
        self.flush_pending(strings)
    }

    fn retain(
        &self,
        versions: &HashSet<ArtifactVersion>,
        strings: &StringPool,
    ) -> Result<(), ArtifactError> {
        self.retain_versions(versions, strings)
    }
}

/// Immutable group of artifacts written together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ArtifactSegment {
    /// The persistent store partition containing this segment.
    pub(super) partition: String,
    /// String texts referenced by the segment records.
    pub(super) strings: Vec<String>,
    /// Artifacts first published by this segment.
    pub(super) records: Vec<ArtifactRecord>,
}

impl ArtifactSegment {
    /// Build one segment from artifact records and their interned strings.
    fn build(
        partition: &str,
        string_pool: &StringPool,
        records: &[ArtifactRecord],
        known_versions: &HashSet<ArtifactVersion>,
    ) -> Result<Self, ArtifactError> {
        // select one new record per exact artifact version
        let mut selected = BTreeMap::new();
        for record in records {
            if known_versions.contains(&record.version) {
                continue;
            }
            selected
                .entry(record.version)
                .or_insert_with(|| record.clone());
        }

        // collect only strings referenced by selected records
        let mut string_ids = selected
            .values()
            .flat_map(|record| record.strings.iter())
            .copied()
            .collect::<Vec<_>>();
        string_ids.sort_unstable();
        string_ids.dedup();

        // resolve every referenced string
        let mut strings = Vec::with_capacity(string_ids.len());
        for string in string_ids {
            let Some(text) = string_pool.get_maybe(string) else {
                return Err(ArtifactError::MissingString { string });
            };
            strings.push(text.to_string());
        }

        Ok(Self {
            partition: partition.to_owned(),
            strings,
            records: selected.into_values().collect(),
        })
    }

    /// Encode this artifact segment.
    pub(super) fn encode(&self) -> Result<Vec<u8>, ArtifactError> {
        let bytes =
            destack_serde::to_vec(self).map_err(|error| ArtifactError::Codec(Box::new(error)))?;
        let actual = bytes.len() as u64;
        if actual > MAX_BLOB_BYTES {
            return Err(ArtifactError::Size {
                limit: MAX_BLOB_BYTES,
                actual,
            });
        }

        Ok(bytes)
    }

    /// Decode one artifact segment.
    pub(super) fn decode(bytes: &[u8]) -> Result<Self, ArtifactError> {
        destack_serde::from_slice(bytes).map_err(|error| ArtifactError::Codec(Box::new(error)))
    }

    /// Insert all records from this segment into an index.
    pub(super) fn insert_into(
        self,
        string_pool: &StringPool,
        records: &mut FxHashMap<ArtifactVersion, ArtifactRecord>,
    ) -> Result<(), ArtifactError> {
        // verify and intern this segment's complete string table
        let mut strings = FxHashMap::default();
        for string in self.strings {
            let string_id = StringId::for_text(&string);
            let string = Arc::<str>::from(string);
            if let Some(existing) = strings.insert(string_id, Arc::clone(&string))
                && existing.as_ref() != string.as_ref()
            {
                return Err(ArtifactError::Invalid(
                    "artifact segment has colliding string identities",
                ));
            }
            string_pool.ensure(string_id, &string);
        }

        // index every artifact record
        for record in self.records {
            for string in &record.strings {
                if !strings.contains_key(string) {
                    return Err(ArtifactError::MissingString { string: *string });
                }
            }
            records.entry(record.version).or_insert(record);
        }

        Ok(())
    }

    /// Build a compact segment from selected artifact records.
    pub(super) fn compact(self, records: Vec<ArtifactRecord>) -> Self {
        let mut retained_strings = HashSet::new();
        for record in &records {
            retained_strings.extend(record.strings.iter().copied());
        }

        let strings = self
            .strings
            .into_iter()
            .filter(|string| retained_strings.contains(&StringId::for_text(string)))
            .collect();

        Self {
            partition: self.partition,
            strings,
            records,
        }
    }
}
