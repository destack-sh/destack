use rustc_hash::FxHashMap;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_core::{StringId, StringPool, stable_hash_bytes_128};
use parking_lot::{Mutex, RwLock};
use serde::{Deserialize, Serialize};

use crate::{
    ARTIFACT_SEGMENT_EXTENSION, ArtifactFlush, ArtifactRecord, ArtifactStore, ArtifactStoreError,
    ArtifactTable, ArtifactVersion, BlobStore, BlobStoreError, MAX_BLOB_BYTES,
    RepositoryStoreLayout,
};

/// Segmented store of canonical artifact records.
#[derive(Debug)]
pub struct SegmentedArtifactStore {
    /// The byte store backing this artifact store.
    store: Arc<dyn BlobStore>,
    /// The repository store layout.
    layout: RepositoryStoreLayout,
    /// The build fingerprint partition this artifact store serves.
    build_fingerprint: String,
    /// Artifact records waiting for segment publication.
    pending: Mutex<Vec<ArtifactRecord>>,
    /// Loaded records keyed by exact artifact version.
    index: RwLock<FxHashMap<ArtifactVersion, IndexedArtifactRecord>>,
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
        Self {
            store,
            layout,
            build_fingerprint: build_fingerprint.into(),
            pending: Mutex::new(Vec::new()),
            index: RwLock::new(HashMap::default()),
            segments: RwLock::new(HashSet::new()),
            is_index_loaded: RwLock::new(false),
        }
    }

    /// Load one exact artifact record.
    fn load_record(
        &self,
        expected: &ArtifactVersion,
        strings: &StringPool,
    ) -> Result<Option<ArtifactRecord>, ArtifactStoreError> {
        // check records written or loaded by this process first
        if let Some(record) = self.load_indexed_record(expected, strings)? {
            return Ok(Some(record));
        }

        // load persistent segment index on first process-local miss
        self.load_index()?;

        let Some(record) = self.load_indexed_record(expected, strings)? else {
            return Ok(None);
        };

        Ok(Some(record))
    }

    /// Load one already indexed artifact record.
    fn load_indexed_record(
        &self,
        expected: &ArtifactVersion,
        strings: &StringPool,
    ) -> Result<Option<ArtifactRecord>, ArtifactStoreError> {
        let index = self.index.read();
        let Some(entry) = index.get(expected) else {
            return Ok(None);
        };

        entry.load_strings(strings)?;

        Ok(Some(entry.record.clone()))
    }

    /// Queue one exact artifact record.
    fn store_record(&self, record: ArtifactRecord) {
        let mut pending = self.pending.lock();

        pending.push(record);
    }

    /// Flush all queued artifact records as one immutable segment.
    fn flush_records(&self, strings: &StringPool) -> Result<ArtifactFlush, ArtifactStoreError> {
        // drain queued records
        let records = {
            let mut pending = self.pending.lock();
            if pending.is_empty() {
                return Ok(ArtifactFlush::default());
            }

            std::mem::take(&mut *pending)
        };

        // build the immutable segment before records are consumed
        let segment = match self.records_segment(records, strings) {
            Ok(segment) => segment,
            Err(error) => {
                let mut pending = self.pending.lock();
                pending.extend(error.records);

                return Err(error.error);
            }
        };

        // publish the segment and requeue when the write did not persist
        match self.write_segment(segment) {
            Ok(flush) => Ok(flush),
            Err(ArtifactSegmentWriteError::Unpublished { segment, error }) => {
                let mut pending = self.pending.lock();
                pending.extend(segment.records);

                Err(error)
            }
            Err(ArtifactSegmentWriteError::Persisted { error }) => Err(error),
        }
    }

    /// Build one immutable segment from owned records.
    fn records_segment(
        &self,
        records: Vec<ArtifactRecord>,
        strings: &StringPool,
    ) -> Result<ArtifactSegment, ArtifactSegmentBuildError> {
        ArtifactSegmentBuilder::new(&self.build_fingerprint, strings)
            .extend(records)
            .build()
    }

    /// Write one immutable segment.
    fn write_segment(
        &self,
        segment: ArtifactSegment,
    ) -> Result<ArtifactFlush, ArtifactSegmentWriteError> {
        // encode and name the segment
        let bytes = match segment.encode() {
            Ok(bytes) => bytes,
            Err(error) => return Err(ArtifactSegmentWriteError::unpublished(segment, error)),
        };
        let path = self.segment_path(&bytes);
        let flush = ArtifactFlush {
            segments: 1,
            records: segment.records.len(),
            strings: segment.strings.len(),
            bytes: bytes.len(),
        };

        // publish the segment and update any loaded index
        if let Err(error) = self.write_segment_bytes(&path, &bytes) {
            return Err(ArtifactSegmentWriteError::unpublished(segment, error));
        }
        self.segments.write().insert(path);
        if let Err(error) = self.index_segment(segment) {
            return Err(ArtifactSegmentWriteError::persisted(error));
        }

        Ok(flush)
    }

    /// Write one immutable segment.
    fn write_segment_bytes(&self, path: &Path, bytes: &[u8]) -> Result<(), ArtifactStoreError> {
        // accept existing identical bytes
        if let Some(existing_bytes) = self.store.read(path)? {
            if existing_bytes != bytes {
                return Err(ArtifactStoreError::Corrupt(
                    "artifact segment hash collision",
                ));
            }

            return Ok(());
        }

        // write new segment bytes
        match self.store.write_once(path, bytes) {
            Ok(()) => {}
            Err(BlobStoreError::AlreadyExists) => {
                let Some(existing_bytes) = self.store.read(path)? else {
                    return Err(ArtifactStoreError::Corrupt(
                        "artifact segment disappeared after write conflict",
                    ));
                };
                if existing_bytes != bytes {
                    return Err(ArtifactStoreError::Corrupt(
                        "artifact segment hash collision",
                    ));
                }
            }
            Err(error) => return Err(error.into()),
        }

        Ok(())
    }

    /// Retain only reachable artifact records.
    fn retain_reachable(
        &self,
        reachable: &HashSet<ArtifactVersion>,
        strings: &StringPool,
    ) -> Result<(), ArtifactStoreError> {
        let _flush = self.flush_records(strings)?;

        let mut retained_index = HashMap::default();
        let mut retained_segments = HashSet::new();

        // rewrite partially live segments under the store lock
        self.with_write_lock(|| {
            for path in self.segment_paths()? {
                let Some(segment) = self.read_segment(&path)? else {
                    continue;
                };
                let records = segment
                    .records
                    .iter()
                    .filter(|record| reachable.contains(&record.version))
                    .cloned()
                    .collect::<Vec<_>>();

                // drop fully unreachable segments
                if records.is_empty() {
                    self.store.remove(&path)?;
                }
                // keep fully reachable segments
                else if records.len() == segment.records.len() {
                    segment.insert_into(&mut retained_index)?;
                    retained_segments.insert(path);

                    continue;
                }
                // compact partially reachable segments
                else {
                    let segment = segment.compact(records);
                    let bytes = segment.encode()?;
                    let compacted_path = self.segment_path(&bytes);

                    self.write_segment_bytes(&compacted_path, &bytes)?;
                    self.store.remove(&path)?;
                    segment.insert_into(&mut retained_index)?;
                    retained_segments.insert(compacted_path);
                }
            }

            Ok(())
        })?;

        // publish retained index after segment pruning
        *self.index.write() = retained_index;
        *self.segments.write() = retained_segments;
        *self.is_index_loaded.write() = true;

        Ok(())
    }

    /// Ensure the in-memory segment index is populated.
    fn load_index(&self) -> Result<(), ArtifactStoreError> {
        if *self.is_index_loaded.read() {
            return Ok(());
        }

        let paths = self.segment_paths()?;
        let known_segments = self.segments.read();
        let paths = paths
            .into_iter()
            .filter(|path| !known_segments.contains(path))
            .collect::<Vec<_>>();
        drop(known_segments);

        if paths.is_empty() {
            *self.is_index_loaded.write() = true;

            return Ok(());
        }

        let mut index = self.index.write();
        let mut is_index_loaded = self.is_index_loaded.write();
        if *is_index_loaded {
            return Ok(());
        }

        let mut known_segments = self.segments.write();
        for path in paths {
            let Some(segment) = self.read_segment(&path)? else {
                continue;
            };

            segment.insert_into(&mut index)?;
            known_segments.insert(path);
        }
        *is_index_loaded = true;

        Ok(())
    }

    /// Add one segment to the process-local index.
    fn index_segment(&self, segment: ArtifactSegment) -> Result<(), ArtifactStoreError> {
        let mut index = self.index.write();

        segment.insert_into(&mut index)
    }

    /// Read one segment file from disk.
    fn read_segment(&self, path: &Path) -> Result<Option<ArtifactSegment>, ArtifactStoreError> {
        // inspect segment size
        let Some(byte_len) = self.store.byte_len(path)? else {
            return Ok(None);
        };
        if byte_len > MAX_BLOB_BYTES {
            return Err(ArtifactStoreError::Size {
                limit: MAX_BLOB_BYTES,
                actual: byte_len,
            });
        }

        // decode segment bytes
        let Some(bytes) = self.store.read(path)? else {
            return Ok(None);
        };
        let segment = ArtifactSegment::decode(&bytes)?;
        if segment.build_fingerprint != self.build_fingerprint {
            return Err(ArtifactStoreError::Corrupt(
                "artifact segment build fingerprint does not match store partition",
            ));
        }

        Ok(Some(segment))
    }

    /// Return all artifact segment paths in this build partition.
    fn segment_paths(&self) -> Result<Vec<PathBuf>, ArtifactStoreError> {
        let root = self.layout.artifact_root(&self.build_fingerprint);
        let paths = self
            .store
            .entries(&root)?
            .into_iter()
            .filter(|path| {
                path.extension().and_then(|extension| extension.to_str())
                    == Some(ARTIFACT_SEGMENT_EXTENSION)
            })
            .collect();

        Ok(paths)
    }

    /// Return the stored segment path for encoded segment bytes.
    fn segment_path(&self, bytes: &[u8]) -> PathBuf {
        let fingerprint = format!("{:032x}", stable_hash_bytes_128(bytes));
        let shard = &fingerprint[0..2];

        self.layout
            .artifact_root(&self.build_fingerprint)
            .join(shard)
            .join(format!("{fingerprint}.{ARTIFACT_SEGMENT_EXTENSION}"))
    }

    /// Run one write operation under the persistent store lock.
    fn with_write_lock<T>(
        &self,
        operation: impl FnOnce() -> Result<T, ArtifactStoreError>,
    ) -> Result<T, ArtifactStoreError> {
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
        expected: &ArtifactVersion,
        strings: &StringPool,
    ) -> Result<Option<ArtifactRecord>, ArtifactStoreError> {
        self.load_record(expected, strings)
    }

    fn store(
        &self,
        version: &ArtifactVersion,
        table: &ArtifactTable,
        strings: &StringPool,
    ) -> Result<(), ArtifactStoreError> {
        let Some(record) = table.record(version, strings)? else {
            return Err(ArtifactStoreError::Corrupt(
                "artifact store cannot persist missing artifact version",
            ));
        };

        self.store_record(record);

        Ok(())
    }

    fn flush(&self, strings: &StringPool) -> Result<ArtifactFlush, ArtifactStoreError> {
        self.flush_records(strings)
    }

    fn retain(
        &self,
        reachable: &HashSet<ArtifactVersion>,
        strings: &StringPool,
    ) -> Result<(), ArtifactStoreError> {
        self.retain_reachable(reachable, strings)
    }
}

/// Immutable group of artifact records written together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ArtifactSegment {
    /// The build fingerprint that produced this segment.
    pub(super) build_fingerprint: String,
    /// String texts referenced by the segment records.
    pub(super) strings: Vec<String>,
    /// Exact artifact records carried by this segment.
    pub(super) records: Vec<ArtifactRecord>,
}

/// Failed artifact segment construction with the original records.
#[derive(Debug)]
struct ArtifactSegmentBuildError {
    /// Original records that were not published.
    records: Vec<ArtifactRecord>,
    /// The build error.
    error: ArtifactStoreError,
}

impl ArtifactSegmentBuildError {
    /// Build one failed segment construction.
    fn new(records: Vec<ArtifactRecord>, error: ArtifactStoreError) -> Self {
        Self { records, error }
    }
}

/// Failed artifact segment write.
#[derive(Debug)]
enum ArtifactSegmentWriteError {
    /// The segment was not persisted.
    Unpublished {
        /// Original segment that was not published.
        segment: ArtifactSegment,
        /// The write error.
        error: ArtifactStoreError,
    },
    /// The segment was persisted, but process-local indexing failed.
    Persisted {
        /// The index error.
        error: ArtifactStoreError,
    },
}

impl ArtifactSegmentWriteError {
    /// Build one unpublished segment error.
    fn unpublished(segment: ArtifactSegment, error: ArtifactStoreError) -> Self {
        Self::Unpublished { segment, error }
    }

    /// Build one persisted segment error.
    fn persisted(error: ArtifactStoreError) -> Self {
        Self::Persisted { error }
    }
}

impl ArtifactSegment {
    /// Encode this artifact segment.
    pub(super) fn encode(&self) -> Result<Vec<u8>, ArtifactStoreError> {
        let bytes = destack_serde::to_vec(self)
            .map_err(|error| ArtifactStoreError::Codec(Box::new(error)))?;
        let actual = bytes.len() as u64;
        if actual > MAX_BLOB_BYTES {
            return Err(ArtifactStoreError::Size {
                limit: MAX_BLOB_BYTES,
                actual,
            });
        }

        Ok(bytes)
    }

    /// Decode one artifact segment.
    pub(super) fn decode(bytes: &[u8]) -> Result<Self, ArtifactStoreError> {
        destack_serde::from_slice(bytes).map_err(|error| ArtifactStoreError::Codec(Box::new(error)))
    }

    /// Insert all records from this segment into an index.
    pub(super) fn insert_into(
        self,
        index: &mut FxHashMap<ArtifactVersion, IndexedArtifactRecord>,
    ) -> Result<(), ArtifactStoreError> {
        let mut strings = HashMap::new();
        for string in self.strings {
            let string_id = StringId::for_text(&string);
            let string = Arc::<str>::from(string);
            if let Some(existing) = strings.insert(string_id, Arc::clone(&string))
                && existing.as_ref() != string.as_ref()
            {
                return Err(ArtifactStoreError::Corrupt(
                    "artifact segment has colliding string identities",
                ));
            }
        }

        // add each record with the segment strings it references
        for record in self.records {
            let record_strings = record
                .strings
                .iter()
                .map(|string| {
                    strings
                        .get(string)
                        .cloned()
                        .ok_or(ArtifactStoreError::MissingString { string: *string })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let entry = IndexedArtifactRecord {
                record,
                strings: record_strings,
            };

            index.entry(entry.record.version).or_insert(entry);
        }

        Ok(())
    }

    /// Build a compact segment carrying only strings referenced by retained records.
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
            build_fingerprint: self.build_fingerprint,
            strings,
            records,
        }
    }
}

/// Artifact record plus its segment string table.
#[derive(Debug, Clone)]
pub(super) struct IndexedArtifactRecord {
    /// The exact artifact record.
    pub(super) record: ArtifactRecord,
    /// The string table needed to decode the record.
    strings: Vec<Arc<str>>,
}

impl IndexedArtifactRecord {
    /// Load strings needed by this indexed record.
    pub(super) fn load_strings(&self, strings: &StringPool) -> Result<(), ArtifactStoreError> {
        for string in &self.strings {
            let string_id = StringId::for_text(string);
            strings.ensure(string_id, string);
        }

        Ok(())
    }
}

/// Builder for one immutable artifact segment.
#[derive(Debug)]
pub(super) struct ArtifactSegmentBuilder<'a> {
    /// The build fingerprint that produced the segment.
    build_fingerprint: String,
    /// The process string pool.
    string_pool: &'a StringPool,
    /// Artifact records to write into the segment.
    records: Vec<ArtifactRecord>,
    /// String ids referenced by the segment records.
    string_ids: Vec<StringId>,
}

impl<'a> ArtifactSegmentBuilder<'a> {
    /// Create one empty segment builder.
    fn new(build_fingerprint: &str, string_pool: &'a StringPool) -> Self {
        Self {
            build_fingerprint: build_fingerprint.to_string(),
            string_pool,
            records: Vec::new(),
            string_ids: Vec::new(),
        }
    }

    /// Add records to this segment.
    fn extend(mut self, records: Vec<ArtifactRecord>) -> Self {
        for record in &records {
            self.string_ids.extend(record.strings.iter().copied());
        }
        self.records = records;

        self
    }

    /// Build the immutable segment.
    fn build(mut self) -> Result<ArtifactSegment, ArtifactSegmentBuildError> {
        self.string_ids.sort_unstable();
        self.string_ids.dedup();

        let mut strings = Vec::with_capacity(self.string_ids.len());
        let records = self.records;
        for string in self.string_ids {
            let Some(text) = self.string_pool.get_maybe(string) else {
                let error = ArtifactStoreError::MissingString { string };

                return Err(ArtifactSegmentBuildError::new(records, error));
            };

            strings.push(text.to_string());
        }

        Ok(ArtifactSegment {
            build_fingerprint: self.build_fingerprint,
            strings,
            records,
        })
    }
}
