use std::collections::HashSet;
use std::io::Cursor;
use std::mem;
use std::path::Path;
use std::sync::Arc;

use destack_artifact::{
    ArtifactError, ArtifactFlush, ArtifactRecord, ArtifactStore, ArtifactVersion, BuildId,
};
use destack_core::{Blob, StringPool};
use parking_lot::{Mutex, RwLock};
use rustc_hash::FxHashMap;

use super::layout::Layout;
use crate::artifact::Segment;
use crate::{BlobStore, DestackLayout};

/// Persistent artifact representation version.
const ARTIFACT_STORE_VERSION: u32 = 10;

/// Disk-backed artifact record storage.
#[derive(Debug)]
pub(crate) struct DiskStore {
    /// Shared immutable Blob storage.
    blobs: Arc<dyn BlobStore>,
    /// Filesystem paths owned by this artifact store.
    layout: Layout,
    /// Persistent partition for this build and record format.
    partition: String,
    /// Artifact records waiting for publication.
    pending: Mutex<Vec<ArtifactRecord>>,
    /// Process-local artifact publication lock.
    publication: Mutex<()>,
    /// Artifact records keyed by exact version.
    records: RwLock<FxHashMap<ArtifactVersion, ArtifactRecord>>,
    /// Segment Blobs already reflected in the local index.
    segments: RwLock<HashSet<Blob>>,
}

impl DiskStore {
    /// Open one artifact store.
    pub(crate) fn new(
        root: &Path,
        layout: &DestackLayout,
        build_id: BuildId,
        blobs: Arc<dyn BlobStore>,
    ) -> Self {
        let partition = format!("v{ARTIFACT_STORE_VERSION}-{build_id}");

        Self {
            blobs,
            layout: Layout::new(root, layout),
            partition,
            pending: Mutex::new(Vec::new()),
            publication: Mutex::new(()),
            records: RwLock::new(FxHashMap::default()),
            segments: RwLock::new(HashSet::new()),
        }
    }

    /// Load one artifact record from local or persistent state.
    fn load_record(
        &self,
        version: &ArtifactVersion,
        strings: &StringPool,
    ) -> Result<Option<ArtifactRecord>, ArtifactError> {
        // consult the process-local index first
        if let Some(record) = self.records.read().get(version).cloned() {
            return Ok(Some(record));
        }

        // discover segments published by this or another process
        self.load_segments(strings)?;

        Ok(self.records.read().get(version).cloned())
    }

    /// Flush queued artifact records as one immutable segment.
    fn flush_pending(&self, strings: &StringPool) -> Result<ArtifactFlush, ArtifactError> {
        // drain queued records
        let records = {
            let mut pending = self.pending.lock();
            if pending.is_empty() {
                return Ok(ArtifactFlush::default());
            }

            mem::take(&mut *pending)
        };

        // serialize publication across processes
        let result = self.flush_records(&records, strings);
        if result.is_err() {
            self.pending.lock().extend(records);
        }

        result
    }

    /// Publish one immutable segment under the artifact store lock.
    fn flush_records(
        &self,
        records: &[ArtifactRecord],
        strings: &StringPool,
    ) -> Result<ArtifactFlush, ArtifactError> {
        let _publication = self.publication.lock();

        // coordinate repository writers across native processes
        #[cfg(not(target_os = "wasi"))]
        let _lock = self.layout.lock()?;

        self.load_segments(strings)?;

        // verify versions already published by this or another process
        let known = self.records.read();
        for record in records {
            if known
                .get(&record.version)
                .is_some_and(|existing| existing != record)
            {
                return Err(ArtifactError::Invalid(
                    "one artifact version has conflicting records",
                ));
            }
        }
        let known_versions = known.keys().copied().collect();
        drop(known);

        // remove identical versions already published by another process
        let segment = Segment::build(&self.partition, strings, records, &known_versions)?;
        if segment.records.is_empty() {
            return Ok(ArtifactFlush::default());
        }

        // publish segment bytes and their reachability reference
        let bytes = segment.encode()?;
        let mut input = Cursor::new(&bytes);
        let blob = self
            .blobs
            .put(&mut input)
            .map_err(|error| ArtifactError::store(error.to_string()))?;
        self.layout.publish(&self.partition, blob)?;

        // make the new records visible locally before releasing the lock
        let artifacts = segment.records.len();
        let string_count = segment.strings.len();
        segment.insert_into(strings, &mut self.records.write())?;
        self.segments.write().insert(blob);

        Ok(ArtifactFlush {
            segments: 1,
            artifacts,
            strings: string_count,
            bytes: bytes.len(),
        })
    }

    /// Add newly referenced segments to the process-local index.
    fn load_segments(&self, strings: &StringPool) -> Result<(), ArtifactError> {
        let known = self.segments.read();
        let segments = self
            .layout
            .segments(&self.partition)?
            .into_iter()
            .filter(|blob| !known.contains(blob))
            .collect::<Vec<_>>();
        drop(known);

        let mut records = self.records.write();
        let mut known = self.segments.write();
        for blob in segments {
            let segment = self.read_segment(blob)?;
            segment.insert_into(strings, &mut records)?;
            known.insert(blob);
        }

        Ok(())
    }

    /// Decode one exact segment Blob.
    fn read_segment(&self, blob: Blob) -> Result<Segment, ArtifactError> {
        let memory = self
            .blobs
            .open(blob)
            .map_err(|error| ArtifactError::store(error.to_string()))?;
        let segment = Segment::decode(memory.bytes())?;
        if segment.partition != self.partition {
            return Err(ArtifactError::Invalid(
                "artifact segment does not match its store partition",
            ));
        }

        Ok(segment)
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

        // serialize compaction with process-local publication
        let _publication = self.publication.lock();

        // coordinate repository writers across native processes
        #[cfg(not(target_os = "wasi"))]
        let _lock = self.layout.lock()?;

        self.load_segments(strings)?;
        let mut retained_records = FxHashMap::default();
        let mut retained_segments = HashSet::new();
        let mut stored_versions = HashSet::new();

        for blob in self.layout.segments(&self.partition)? {
            let segment = self.read_segment(blob)?;
            let records = segment
                .records
                .iter()
                .filter(|record| {
                    versions.contains(&record.version) && stored_versions.insert(record.version)
                })
                .cloned()
                .collect::<Vec<_>>();

            // drop references to fully unreachable segments
            if records.is_empty() {
                self.layout.remove(&self.partition, blob)?;
            }
            // retain segments whose complete record set remains reachable
            else if records.len() == segment.records.len() {
                segment.insert_into(strings, &mut retained_records)?;
                retained_segments.insert(blob);
            }
            // publish compacted segment bytes before replacing the old reference
            else {
                let segment = segment.retain(records);
                let bytes = segment.encode()?;
                let mut input = Cursor::new(bytes);
                let compacted = self
                    .blobs
                    .put(&mut input)
                    .map_err(|error| ArtifactError::store(error.to_string()))?;

                self.layout.publish(&self.partition, compacted)?;
                self.layout.remove(&self.partition, blob)?;
                segment.insert_into(strings, &mut retained_records)?;
                retained_segments.insert(compacted);
            }
        }

        *self.records.write() = retained_records;
        *self.segments.write() = retained_segments;

        Ok(())
    }
}

impl ArtifactStore for DiskStore {
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
