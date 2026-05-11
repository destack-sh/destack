use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::trace::{
    TraceCheckpointIndex, TraceChunkIndex, TraceCursor, TraceHeader, TraceRecord, TraceTrailer,
};
use crate::runtime::world::BranchId;
use destack_core::{FNV_OFFSET_BASIS_128, fnv1a_128_update};
use postcard::experimental::serialized_size;

use super::chunk::{TraceChunk, TraceChunkChain};

/// Default maximum number of events in a chunk.
const DEFAULT_MAX_EVENTS_PER_CHUNK: usize = 1024;
/// Default maximum chunk size in bytes.
const DEFAULT_MAX_CHUNK_BYTES: u64 = 4 * 1024 * 1024;

/// Sequence number for events within a trace log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TraceSequence(u64);

impl TraceSequence {
    /// Create a new trace sequence number.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the raw sequence number.
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Return the next sequence number.
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

/// One mutable tail for a live trace log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TraceTail {
    /// The current mutable active chunk.
    active: TraceChunk,
    /// The completed local chunks not yet folded into shared history.
    sealed: Vec<TraceChunk>,
}

impl TraceTail {
    /// Create one empty tail at the given chunk index and sequence.
    fn new(index: u32, sequence_start: TraceSequence) -> Self {
        Self {
            active: TraceChunk::new(index, sequence_start),
            sealed: Vec::new(),
        }
    }

    /// Return the number of materialized tail chunks.
    fn chunk_count(&self) -> usize {
        let active_chunk_count = usize::from(!self.active.is_empty());

        self.sealed.len() + active_chunk_count
    }
}

/// One live trace-log state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TraceState {
    /// Trace log header metadata.
    header: TraceHeader,
    /// Trace log trailer metadata.
    trailer: TraceTrailer,
    /// Maximum number of events per chunk.
    max_events_per_chunk: usize,
    /// Maximum chunk size in bytes.
    max_chunk_bytes: u64,
    /// Next sequence number to assign.
    next_sequence: TraceSequence,
    /// Next chunk offset for trailer entries.
    next_offset: u64,
    /// Shared immutable chunk history.
    head: Option<Arc<TraceChunkChain>>,
    /// Mutable local append frontier.
    tail: TraceTail,
}

impl TraceState {
    /// Return the active branch identifier.
    pub(super) fn branch_id(&self) -> BranchId {
        self.header.branch_id
    }

    /// Return the next trace sequence number.
    pub(super) fn next_sequence(&self) -> TraceSequence {
        self.next_sequence
    }

    /// Return the trace log trailer.
    pub(super) fn trailer(&self) -> &TraceTrailer {
        &self.trailer
    }

    /// Return the number of shared immutable chunks.
    pub(super) fn head_chunk_count(&self) -> usize {
        self.head
            .as_ref()
            .map(|chain| chain.chunk_count as usize)
            .unwrap_or(0)
    }

    /// Return the shared immutable trace head.
    pub(super) fn head(&self) -> Option<&Arc<TraceChunkChain>> {
        self.head.as_ref()
    }

    /// Return the sealed local tail chunks.
    pub(super) fn sealed_tail(&self) -> &[TraceChunk] {
        &self.tail.sealed
    }

    /// Return the active mutable chunk when it stores events.
    pub(super) fn active_chunk(&self) -> Option<&TraceChunk> {
        (!self.tail.active.is_empty()).then_some(&self.tail.active)
    }

    /// Return the total number of visible chunks.
    pub(super) fn chunk_count(&self) -> usize {
        self.head_chunk_count() + self.tail.chunk_count()
    }

    /// Return the next chunk index for one new active chunk.
    fn next_chunk_index(&self) -> u32 {
        self.chunk_count() as u32
    }

    /// Append one trailer entry for the active chunk if needed.
    fn ensure_active_trailer_entry(&mut self) {
        // skip empty active chunks
        if self.tail.active.is_empty() {
            return;
        }

        let active_index = self.tail.active.header.index;
        let is_entry_present = self
            .trailer
            .chunks
            .last()
            .is_some_and(|entry| entry.index == active_index);

        // keep one trailer entry per materialized chunk
        if is_entry_present {
            return;
        }

        self.trailer.chunks.push(TraceChunkIndex {
            index: active_index,
            offset: self.next_offset,
            length: 0,
            checksum: 0,
        });
    }

    /// Refresh the trailer entry for the active chunk.
    fn update_active_trailer_entry(&mut self) {
        let Some(entry) = self.trailer.chunks.last_mut() else {
            return;
        };

        entry.length = self.tail.active.header.byte_length;
        entry.checksum = self.tail.active.header.checksum;
    }

    /// Finalize one non-empty active chunk into the local sealed tail.
    fn seal_active_chunk(&mut self, next_sequence_start: TraceSequence) {
        // skip sealing empty chunks
        if self.tail.active.is_empty() {
            self.tail.active = TraceChunk::new(self.next_chunk_index(), next_sequence_start);
            return;
        }

        // keep the trailer synchronized before moving the chunk
        self.update_active_trailer_entry();
        self.next_offset = self
            .next_offset
            .saturating_add(self.tail.active.header.byte_length);

        let next_index = self.next_chunk_index();
        let sealed_chunk = std::mem::replace(
            &mut self.tail.active,
            TraceChunk::new(next_index, next_sequence_start),
        );
        self.tail.sealed.push(sealed_chunk);
    }

    /// Fold the local sealed tail into shared immutable history.
    fn materialize_tail(&mut self) {
        // keep the active chunk synchronized before materialization
        self.seal_active_chunk(self.next_sequence);

        // nothing to do when the local tail is already empty
        if self.tail.sealed.is_empty() {
            return;
        }

        let parent = self.head.clone();
        let chunks = std::mem::take(&mut self.tail.sealed);
        let chain = Arc::new(TraceChunkChain::new(parent, chunks));
        let next_index = chain.chunk_count;

        self.head = Some(chain);
        self.tail.active = TraceChunk::new(next_index, self.next_sequence);
    }
}

/// Materialized trace-log image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TraceLogImage {
    /// Trace log header metadata.
    header: TraceHeader,
    /// Trace log trailer metadata.
    trailer: TraceTrailer,
    /// Maximum number of events per chunk.
    max_events_per_chunk: usize,
    /// Maximum chunk size in bytes.
    max_chunk_bytes: u64,
    /// Next sequence number to assign.
    next_sequence: TraceSequence,
    /// Next chunk offset for trailer entries.
    next_offset: u64,
    /// Shared immutable chunk history for this branch image.
    head: Option<Arc<TraceChunkChain>>,
}

#[allow(dead_code)]
impl TraceLogImage {
    /// Return the trace header for this image.
    pub(crate) fn header(&self) -> TraceHeader {
        self.header.clone()
    }

    /// Return the active branch identifier.
    pub(super) fn branch_id(&self) -> BranchId {
        self.header.branch_id
    }

    /// Return the next trace sequence number.
    pub(super) fn next_sequence(&self) -> TraceSequence {
        self.next_sequence
    }

    /// Return the total number of stored chunks.
    pub(super) fn chunk_count(&self) -> usize {
        self.head
            .as_ref()
            .map(|chain| chain.chunk_count as usize)
            .unwrap_or(0)
    }

    /// Report whether this image shares the same immutable trace head.
    pub(crate) fn shares_head_with(&self, other: &Self) -> bool {
        match (&self.head, &other.head) {
            (Some(left), Some(right)) => Arc::ptr_eq(left, right),
            (None, None) => true,
            _ => false,
        }
    }

    /// Report whether this image extends the other image's immutable trace head.
    pub(crate) fn extends_head_of(&self, other: &Self) -> bool {
        match (&self.head, &other.head) {
            (_, None) => true,
            (Some(left), Some(right)) => TraceChunkChain::contains(left, right),
            (None, Some(_)) => false,
        }
    }
}

/// Trace log for deterministic execution.
#[derive(Debug, Clone)]
pub struct TraceLog {
    // NOTE #Incomplete: persist chunks to disk and stream across threads
    /// Shared trace log state.
    state: Arc<Mutex<TraceState>>,
}

impl TraceLog {
    /// Create a trace log with an explicit header.
    pub fn new(mut header: TraceHeader) -> Self {
        // normalize chunk limits
        if header.max_events_per_chunk == 0 {
            header.max_events_per_chunk = DEFAULT_MAX_EVENTS_PER_CHUNK as u32;
        }
        if header.max_chunk_bytes == 0 {
            header.max_chunk_bytes = DEFAULT_MAX_CHUNK_BYTES;
        }

        let max_events_per_chunk = header.max_events_per_chunk as usize;
        let max_chunk_bytes = header.max_chunk_bytes;
        let next_sequence = TraceSequence::new(0);

        Self {
            state: Arc::new(Mutex::new(TraceState {
                header,
                trailer: TraceTrailer::default(),
                max_events_per_chunk,
                max_chunk_bytes,
                next_sequence,
                next_offset: 0,
                head: None,
                tail: TraceTail::new(0, next_sequence),
            })),
        }
    }

    /// Return the trace log header.
    pub fn header(&self) -> TraceHeader {
        let state = self.state.lock();

        state.header.clone()
    }

    /// Return the trace log trailer.
    pub fn trailer(&self) -> TraceTrailer {
        let state = self.state.lock();
        let mut trailer = state.trailer.clone();

        trailer.log_hash = compute_log_hash(&trailer.chunks, &trailer.checkpoints);
        trailer
    }

    /// Return the current branch identifier.
    pub fn branch_id(&self) -> BranchId {
        let state = self.state.lock();

        state.header.branch_id
    }

    /// Set the current branch identifier on the trace header.
    pub(crate) fn set_branch_id(&self, branch_id: BranchId) {
        let mut state = self.state.lock();
        state.header.branch_id = branch_id;
    }

    /// Create a trace cursor for this log.
    pub fn reader(&self) -> TraceCursor {
        TraceCursor::new(self.state.clone())
    }

    /// Return the next sequence number.
    pub fn next_sequence(&self) -> TraceSequence {
        let state = self.state.lock();

        state.next_sequence
    }

    /// Capture one full trace-log image.
    pub(crate) fn image(&self) -> TraceLogImage {
        let mut state = self.state.lock();

        // materialize the current tail so captured images share immutable chains
        state.materialize_tail();

        TraceLogImage {
            header: state.header.clone(),
            trailer: state.trailer.clone(),
            max_events_per_chunk: state.max_events_per_chunk,
            max_chunk_bytes: state.max_chunk_bytes,
            next_sequence: state.next_sequence,
            next_offset: state.next_offset,
            head: state.head.clone(),
        }
    }

    /// Restore one full trace-log image.
    pub(crate) fn restore_image(&self, image: TraceLogImage) {
        let mut current = self.state.lock();

        // restore one fresh empty tail after the captured immutable history
        let tail = TraceTail::new(image.chunk_count() as u32, image.next_sequence);

        *current = TraceState {
            header: image.header,
            trailer: image.trailer,
            max_events_per_chunk: image.max_events_per_chunk,
            max_chunk_bytes: image.max_chunk_bytes,
            next_sequence: image.next_sequence,
            next_offset: image.next_offset,
            head: image.head,
            tail,
        };
    }

    /// Record one trace record in the log.
    pub(crate) fn record_event(&self, event: TraceRecord) -> RuntimeResult<TraceSequence> {
        // compute the encoded size before touching trace state
        let encoded_len = serialized_size(&event).map_err(|_| {
            RuntimeError::TraceEncodeFailed {
                name: "event".to_string(),
            }
            .boxed()
        })? as u64;

        let mut state = self.state.lock();
        let sequence = state.next_sequence;
        state.next_sequence = state.next_sequence.next();

        // rotate the active chunk before appending when it is full
        let is_rotation_required = !state.tail.active.is_empty()
            && state.tail.active.should_rotate_for_event(
                encoded_len,
                state.max_events_per_chunk,
                state.max_chunk_bytes,
            );
        if is_rotation_required {
            state.seal_active_chunk(sequence);
        }

        // append the encoded event to the active chunk
        let chunk = &mut state.tail.active;
        let start = chunk.data.len();
        let end = start + encoded_len as usize;
        chunk.data.resize(end, 0);
        let encoded_len = postcard::to_slice(&event, &mut chunk.data[start..end])
            .map_err(|_| {
                RuntimeError::TraceEncodeFailed {
                    name: "event".to_string(),
                }
                .boxed()
            })?
            .len();
        let encoded_end = start + encoded_len;

        chunk.update_checksum_for_range(start, encoded_end);
        chunk.data.truncate(encoded_end);
        chunk.header.byte_length = chunk.data.len() as u64;
        chunk.event_lengths.push(encoded_len as u32);
        chunk.header.event_count = chunk.event_lengths.len() as u32;
        chunk.header.sequence_end = sequence;

        // keep trailer metadata aligned with the active chunk
        state.ensure_active_trailer_entry();
        state.update_active_trailer_entry();

        Ok(sequence)
    }

    /// Record a checkpoint index entry in trailer metadata.
    pub fn record_checkpoint(&self, mut checkpoint: TraceCheckpointIndex) -> RuntimeResult<()> {
        // align the checkpoint with the next trace sequence
        let sequence = self.next_sequence();
        checkpoint.sequence = sequence;

        self.record_checkpoint_exact(checkpoint)
    }

    /// Record a checkpoint index entry with an explicit sequence boundary.
    pub fn record_checkpoint_exact(&self, checkpoint: TraceCheckpointIndex) -> RuntimeResult<()> {
        let sequence = checkpoint.sequence.get();

        // append the checkpoint entry to trailer metadata
        let mut state = self.state.lock();
        let insert_index = state
            .trailer
            .checkpoints
            .partition_point(|entry| entry.sequence.get() <= sequence);
        state.trailer.checkpoints.insert(insert_index, checkpoint);

        Ok(())
    }
}

pub(super) fn compute_log_hash(
    chunks: &[TraceChunkIndex],
    checkpoints: &[TraceCheckpointIndex],
) -> u128 {
    let mut hash = FNV_OFFSET_BASIS_128;
    for chunk in chunks {
        hash = fnv1a_128_update(hash, &chunk.index.to_le_bytes());
        hash = fnv1a_128_update(hash, &chunk.offset.to_le_bytes());
        hash = fnv1a_128_update(hash, &chunk.length.to_le_bytes());
        hash = fnv1a_128_update(hash, &chunk.checksum.to_le_bytes());
    }

    for checkpoint in checkpoints {
        hash = fnv1a_128_update(hash, &checkpoint.checkpoint_id.get().to_le_bytes());
        hash = fnv1a_128_update(hash, &checkpoint.revision.get().to_le_bytes());
        hash = fnv1a_128_update(hash, &checkpoint.sequence.get().to_le_bytes());
        hash = fnv1a_128_update(hash, &checkpoint.size_bytes.to_le_bytes());
        hash = fnv1a_128_update(hash, &checkpoint.hash.to_le_bytes());
        hash = fnv1a_128_update(hash, checkpoint.path.as_bytes());
    }

    hash
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::runtime::time::Instant;
    use crate::runtime::trace::{EnvironmentConfig, Outcome, TraceRecord};
    use crate::runtime::world::{CheckpointId, Revision};

    /// Build one explicit trace header for log tests.
    fn test_trace_header() -> TraceHeader {
        TraceHeader::new(EnvironmentConfig::default())
    }

    /// Capture one trace image should materialize the local tail into shared history.
    #[test]
    fn test_image_materializes_shared_history() {
        let log = TraceLog::new(test_trace_header());
        log.record_event(TraceRecord::Outcome(Outcome::TimeAdvance(Instant::new(1))))
            .expect("record tick");

        let snapshot = log.image();
        let state = log.state.lock();

        assert_eq!(snapshot.chunk_count(), 1);
        assert_eq!(state.head_chunk_count(), 1);
        assert!(state.tail.active.is_empty());
        assert!(state.tail.sealed.is_empty());
        assert!(Arc::ptr_eq(
            state.head.as_ref().expect("state head"),
            snapshot.head.as_ref().expect("snapshot head")
        ));
    }

    /// Appending after one captured trace image should keep the shared head stable.
    #[test]
    fn test_record_after_image_keeps_shared_head() {
        let log = TraceLog::new(test_trace_header());
        log.record_event(TraceRecord::Outcome(Outcome::TimeAdvance(Instant::new(1))))
            .expect("record first tick");
        let snapshot = log.image();
        let snapshot_head = snapshot.head.clone().expect("snapshot head");

        log.record_event(TraceRecord::Outcome(Outcome::TimeAdvance(Instant::new(2))))
            .expect("record second tick");

        let state = log.state.lock();
        assert!(Arc::ptr_eq(
            state.head.as_ref().expect("state head"),
            &snapshot_head
        ));
        assert_eq!(state.tail.active.header.event_count, 1);
        assert_eq!(
            state.tail.active.header.sequence_start,
            TraceSequence::new(1)
        );
    }

    /// Recording one checkpoint with an explicit sequence should preserve checkpoint order.
    #[test]
    fn test_record_checkpoint_exact_orders_by_sequence() {
        let log = TraceLog::new(test_trace_header());

        log.record_checkpoint_exact(TraceCheckpointIndex {
            checkpoint_id: CheckpointId::new(2),
            revision: Revision::new(2),
            sequence: TraceSequence::new(5),
            path: "memory://checkpoint/2".to_string(),
            hash: 22,
            size_bytes: 2,
        })
        .expect("record later checkpoint");
        log.record_checkpoint_exact(TraceCheckpointIndex {
            checkpoint_id: CheckpointId::new(1),
            revision: Revision::new(1),
            sequence: TraceSequence::new(3),
            path: "memory://checkpoint/1".to_string(),
            hash: 11,
            size_bytes: 1,
        })
        .expect("record earlier checkpoint");

        let trailer = log.trailer();
        let sequences = trailer
            .checkpoints
            .iter()
            .map(|checkpoint| checkpoint.sequence.get())
            .collect::<Vec<_>>();

        assert_eq!(sequences, vec![3, 5]);
    }

    /// Restoring one captured trace image should keep existing cursors usable.
    #[test]
    fn test_restore_image_keeps_cursor_validation_consistent() {
        let log = TraceLog::new(test_trace_header());
        log.record_event(TraceRecord::Outcome(Outcome::TimeAdvance(Instant::new(1))))
            .expect("record first tick");
        let mut cursor = log.reader();
        let cursor_image = cursor.capture_image();
        let image = log.image();

        log.record_event(TraceRecord::Outcome(Outcome::TimeAdvance(Instant::new(2))))
            .expect("record second tick");
        log.restore_image(image);
        cursor.restore_image(cursor_image).expect("restore cursor");

        let event = cursor.next_event().expect("read restored event");
        match event {
            Some(TraceRecord::Outcome(Outcome::TimeAdvance(deadline))) => {
                assert_eq!(deadline, Instant::new(1));
            }
            other => panic!("unexpected restored event: {other:?}"),
        }

        assert!(
            cursor
                .next_event()
                .expect("read end of restored log")
                .is_none()
        );
    }

    /// Seeking one cursor should reposition it at the requested sequence boundary.
    #[test]
    fn test_cursor_seek_sequence_repositions_reader() {
        let log = TraceLog::new(test_trace_header());
        log.record_event(TraceRecord::Outcome(Outcome::TimeAdvance(Instant::new(1))))
            .expect("record first tick");
        log.record_event(TraceRecord::Outcome(Outcome::TimeAdvance(Instant::new(2))))
            .expect("record second tick");

        let mut cursor = log.reader();
        cursor
            .seek_sequence(TraceSequence::new(1))
            .expect("seek cursor");

        let event = cursor.next_event().expect("read sought event");
        match event {
            Some(TraceRecord::Outcome(Outcome::TimeAdvance(deadline))) => {
                assert_eq!(deadline, Instant::new(2));
            }
            other => panic!("unexpected sought event: {other:?}"),
        }

        assert_eq!(cursor.sequence(), TraceSequence::new(2));
    }
}
