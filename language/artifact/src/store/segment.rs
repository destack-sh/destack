use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use destack_core::{StringId, StringPool, stable_hash_bytes_128};
use parking_lot::{Mutex, RwLock};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

use crate::{
    ARTIFACT_SEGMENT_EXTENSION, ARTIFACT_STORE_VERSION, ArtifactBindingRecord, ArtifactError,
    ArtifactFlush, ArtifactInput, ArtifactRecord, ArtifactResultRecord, ArtifactStore,
    ArtifactVersion, BlobStore, BlobStoreError, MAX_BLOB_BYTES, RepositoryStoreLayout,
};

/// Segmented store of artifact bindings and results.
#[derive(Debug)]
pub struct SegmentedArtifactStore {
    /// The byte store backing this artifact store.
    store: Arc<dyn BlobStore>,
    /// The repository store layout.
    layout: RepositoryStoreLayout,
    /// The toolchain build fingerprint used by artifact inputs.
    build_fingerprint: String,
    /// The persistent store partition for this build and record format.
    partition: String,
    /// Artifact bindings and results waiting for publication.
    pending: Mutex<Vec<ArtifactRecord>>,
    /// Loaded results keyed by exact artifact version.
    results: RwLock<FxHashMap<ArtifactVersion, ArtifactResultRecord>>,
    /// Loaded bindings keyed by artifact input.
    bindings: RwLock<FxHashMap<ArtifactInput, ArtifactBindingRecord>>,
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
            build_fingerprint,
            partition,
            pending: Mutex::new(Vec::new()),
            results: RwLock::new(FxHashMap::default()),
            bindings: RwLock::new(FxHashMap::default()),
            segments: RwLock::new(HashSet::new()),
            is_index_loaded: RwLock::new(false),
        }
    }

    /// Load one artifact binding by input.
    fn load_binding_record(
        &self,
        input: &ArtifactInput,
        strings: &StringPool,
    ) -> Result<Option<ArtifactBindingRecord>, ArtifactError> {
        // check bindings written or loaded by this process first
        if let Some(binding) = self.load_binding_from_index(input) {
            binding.verify(&self.build_fingerprint)?;

            return Ok(Some(binding));
        }

        // load persistent segment index on first process-local miss
        self.load_index(strings)?;

        let Some(binding) = self.load_binding_from_index(input) else {
            return Ok(None);
        };
        binding.verify(&self.build_fingerprint)?;

        Ok(Some(binding))
    }

    /// Load one exact artifact result.
    fn load_result_record(
        &self,
        expected: &ArtifactVersion,
        strings: &StringPool,
    ) -> Result<Option<ArtifactResultRecord>, ArtifactError> {
        // check results written or loaded by this process first
        if let Some(record) = self.load_result_from_index(expected) {
            return Ok(Some(record));
        }

        // load persistent segment index on first process-local miss
        self.load_index(strings)?;

        Ok(self.load_result_from_index(expected))
    }

    /// Load one artifact result from the process-local index.
    fn load_result_from_index(&self, expected: &ArtifactVersion) -> Option<ArtifactResultRecord> {
        let results = self.results.read();
        let result = results.get(expected)?;

        Some(result.clone())
    }

    /// Load one artifact binding from the process-local index.
    fn load_binding_from_index(&self, input: &ArtifactInput) -> Option<ArtifactBindingRecord> {
        let bindings = self.bindings.read();
        let binding = bindings.get(input)?;

        Some(binding.clone())
    }

    /// Flush queued artifact bindings and results as one immutable segment.
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
            let known_results = self.results.read().keys().copied().collect::<HashSet<_>>();
            let known_bindings = self
                .bindings
                .read()
                .iter()
                .map(|(input, binding)| (*input, binding.version))
                .collect::<FxHashMap<_, _>>();
            let segment = ArtifactSegment::build(
                &self.partition,
                strings,
                &records,
                &known_results,
                &known_bindings,
            )?;
            if segment.results.is_empty() && segment.bindings.is_empty() {
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
            results: segment.results.len(),
            bindings: segment.bindings.len(),
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

    /// Retain only selected artifact input records.
    fn retain_inputs(
        &self,
        inputs: &HashSet<ArtifactInput>,
        strings: &StringPool,
    ) -> Result<(), ArtifactError> {
        // discard queued records that are no longer selected
        self.pending
            .lock()
            .retain(|record| inputs.contains(&record.binding.input));
        let _flush = self.flush_pending(strings)?;

        // compact binding and result rows under the store lock
        self.with_write_lock(|| {
            self.load_new_segments(strings)?;
            let retained_versions = inputs
                .iter()
                .filter_map(|input| {
                    self.bindings
                        .read()
                        .get(input)
                        .map(|binding| binding.version)
                })
                .collect::<HashSet<_>>();
            let mut retained_results = FxHashMap::default();
            let mut retained_bindings = FxHashMap::default();
            let mut retained_segments = HashSet::new();
            let mut stored_results = HashSet::new();

            for path in self.segment_paths()? {
                let Some(segment) = self.read_segment(&path)? else {
                    continue;
                };
                let bindings = segment
                    .bindings
                    .iter()
                    .filter(|binding| inputs.contains(&binding.input))
                    .cloned()
                    .collect::<Vec<_>>();
                let results = segment
                    .results
                    .iter()
                    .filter(|result| {
                        retained_versions.contains(&result.version)
                            && stored_results.insert(result.version)
                    })
                    .cloned()
                    .collect::<Vec<_>>();

                // drop fully unreachable segments
                if bindings.is_empty() && results.is_empty() {
                    self.store.remove(&path)?;
                }
                // keep fully reachable segments
                else if bindings.len() == segment.bindings.len()
                    && results.len() == segment.results.len()
                {
                    segment.insert_into(strings, &mut retained_results, &mut retained_bindings)?;
                    retained_segments.insert(path);

                    continue;
                }
                // compact partially reachable segments
                else {
                    let segment = segment.compact(bindings, results);
                    let bytes = segment.encode()?;
                    let compacted_path = self.segment_path(&bytes);

                    self.write_segment_bytes(&compacted_path, &bytes)?;
                    if compacted_path != path {
                        self.store.remove(&path)?;
                    }
                    segment.insert_into(strings, &mut retained_results, &mut retained_bindings)?;
                    retained_segments.insert(compacted_path);
                }
            }

            // require every retained binding result after compaction
            for binding in retained_bindings.values() {
                if !retained_results.contains_key(&binding.version) {
                    return Err(ArtifactError::Invalid(
                        "retained artifact binding has no persisted result",
                    ));
                }
            }

            *self.results.write() = retained_results;
            *self.bindings.write() = retained_bindings;
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

        let mut results = self.results.write();
        let mut bindings = self.bindings.write();
        let mut known_segments = self.segments.write();
        for path in paths {
            let Some(segment) = self.read_segment(&path)? else {
                continue;
            };

            segment.insert_into(strings, &mut results, &mut bindings)?;
            known_segments.insert(path);
        }

        // require every persisted binding to name one persisted result
        for binding in bindings.values() {
            if !results.contains_key(&binding.version) {
                return Err(ArtifactError::Invalid(
                    "persisted artifact binding has no result",
                ));
            }
        }

        Ok(())
    }

    /// Add one segment to the process-local index.
    fn index_segment(
        &self,
        segment: ArtifactSegment,
        strings: &StringPool,
    ) -> Result<(), ArtifactError> {
        let mut results = self.results.write();
        let mut bindings = self.bindings.write();

        segment.insert_into(strings, &mut results, &mut bindings)
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
    fn load_binding(
        &self,
        input: &ArtifactInput,
        strings: &StringPool,
    ) -> Result<Option<ArtifactBindingRecord>, ArtifactError> {
        self.load_binding_record(input, strings)
    }

    fn load_result(
        &self,
        expected: &ArtifactVersion,
        strings: &StringPool,
    ) -> Result<Option<ArtifactResultRecord>, ArtifactError> {
        self.load_result_record(expected, strings)
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
        inputs: &HashSet<ArtifactInput>,
        strings: &StringPool,
    ) -> Result<(), ArtifactError> {
        self.retain_inputs(inputs, strings)
    }
}

/// Immutable group of artifact bindings and results written together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ArtifactSegment {
    /// The persistent store partition containing this segment.
    pub(super) partition: String,
    /// String texts referenced by the segment records.
    pub(super) strings: Vec<String>,
    /// Artifact results first published by this segment.
    pub(super) results: Vec<ArtifactResultRecord>,
    /// Artifact input bindings carried by this segment.
    pub(super) bindings: Vec<ArtifactBindingRecord>,
}

impl ArtifactSegment {
    /// Build one segment from artifact records and their interned strings.
    fn build(
        partition: &str,
        string_pool: &StringPool,
        records: &[ArtifactRecord],
        known_results: &HashSet<ArtifactVersion>,
        known_bindings: &FxHashMap<ArtifactInput, ArtifactVersion>,
    ) -> Result<Self, ArtifactError> {
        // select one new result and binding per exact identity
        let mut results = BTreeMap::new();
        let mut bindings = BTreeMap::<ArtifactInput, ArtifactBindingRecord>::new();
        for record in records {
            if let Some(existing) = known_bindings.get(&record.binding.input) {
                if *existing != record.binding.version {
                    return Err(ArtifactError::Nondeterministic {
                        input: Box::new(record.binding.input),
                        existing: Box::new(*existing),
                        produced: Box::new(record.binding.version),
                    });
                }

                continue;
            }
            if !known_results.contains(&record.result.version) {
                results
                    .entry(record.result.version)
                    .or_insert_with(|| record.result.clone());
            }
            if let Some(existing) = bindings.get(&record.binding.input)
                && existing.version != record.binding.version
            {
                return Err(ArtifactError::Nondeterministic {
                    input: Box::new(record.binding.input),
                    existing: Box::new(existing.version),
                    produced: Box::new(record.binding.version),
                });
            }
            bindings
                .entry(record.binding.input)
                .or_insert_with(|| record.binding.clone());
        }

        // collect only strings referenced by selected rows
        let mut string_ids = results
            .values()
            .flat_map(|result| result.strings.iter())
            .chain(bindings.values().flat_map(|binding| binding.strings.iter()))
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
            results: results.into_values().collect(),
            bindings: bindings.into_values().collect(),
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
        results: &mut FxHashMap<ArtifactVersion, ArtifactResultRecord>,
        bindings: &mut FxHashMap<ArtifactInput, ArtifactBindingRecord>,
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

        // index result rows independently from their producing inputs
        for result in self.results {
            for string in &result.strings {
                if !strings.contains_key(string) {
                    return Err(ArtifactError::MissingString { string: *string });
                }
            }
            results.entry(result.version).or_insert(result);
        }

        // index each input binding and reject nondeterministic records
        for binding in self.bindings {
            for string in &binding.strings {
                if !strings.contains_key(string) {
                    return Err(ArtifactError::MissingString { string: *string });
                }
            }
            if let Some(existing) = bindings.get(&binding.input)
                && existing.version != binding.version
            {
                return Err(ArtifactError::Nondeterministic {
                    input: Box::new(binding.input),
                    existing: Box::new(existing.version),
                    produced: Box::new(binding.version),
                });
            }
            bindings.entry(binding.input).or_insert(binding);
        }

        Ok(())
    }

    /// Build a compact segment from selected bindings and results.
    pub(super) fn compact(
        self,
        bindings: Vec<ArtifactBindingRecord>,
        results: Vec<ArtifactResultRecord>,
    ) -> Self {
        let mut retained_strings = HashSet::new();
        for result in &results {
            retained_strings.extend(result.strings.iter().copied());
        }
        for binding in &bindings {
            retained_strings.extend(binding.strings.iter().copied());
        }

        let strings = self
            .strings
            .into_iter()
            .filter(|string| retained_strings.contains(&StringId::for_text(string)))
            .collect();

        Self {
            partition: self.partition,
            strings,
            results,
            bindings,
        }
    }
}
