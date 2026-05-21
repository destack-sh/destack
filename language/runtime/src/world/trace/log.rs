use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::world::BranchId;
use crate::world::trace::{
    TRACE_DEFAULT_MAX_CHUNK_BYTES, TRACE_DEFAULT_MAX_EVENTS_PER_CHUNK, TraceCheckpointIndex,
    TraceCursor, TraceHeader, TraceRecord, TraceTrailer,
};
use postcard::experimental::serialized_size;

use super::chunk::{TRACE_EVENT_LENGTH_BYTES, TraceChunk, TracePrefix};
use super::file::{TraceFile, build_trailer};

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
    /// Create one empty tail at the given sequence.
    fn new(sequence_start: TraceSequence) -> Self {
        Self {
            active: TraceChunk::new(sequence_start),
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
    /// Checkpoints anchored in this trace.
    checkpoints: Vec<TraceCheckpointIndex>,
    /// Next sequence number to assign.
    next_sequence: TraceSequence,
    /// Shared immutable trace prefix.
    head: Option<Arc<TracePrefix>>,
    /// Mutable local append frontier.
    tail: TraceTail,
}

impl TraceState {
    /// Return the active branch identifier.
    pub(super) fn branch_id(&self) -> BranchId {
        self.header.branch_id
    }

    /// Return the next trace sequence number.
    pub(crate) fn next_sequence(&self) -> TraceSequence {
        self.next_sequence
    }

    /// Return the number of shared immutable chunks.
    pub(super) fn head_chunk_count(&self) -> usize {
        self.head
            .as_ref()
            .map(|prefix| prefix.chunk_count as usize)
            .unwrap_or(0)
    }

    /// Return the shared immutable trace head.
    pub(super) fn head(&self) -> Option<&Arc<TracePrefix>> {
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

    /// Finalize one non-empty active chunk into the local sealed tail.
    fn seal_active_chunk(&mut self, next_sequence_start: TraceSequence) {
        // skip sealing empty chunks
        if self.tail.active.is_empty() {
            self.tail.active = TraceChunk::new(next_sequence_start);
            return;
        }

        let sealed_chunk =
            std::mem::replace(&mut self.tail.active, TraceChunk::new(next_sequence_start));
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
        let prefix = Arc::new(TracePrefix::new(parent, chunks));

        self.head = Some(prefix);
        self.tail.active = TraceChunk::new(self.next_sequence);
    }

    /// Return the visible chunks in trace-file order.
    fn chunks(&self) -> Vec<TraceChunk> {
        let mut chunks = Vec::with_capacity(self.chunk_count());
        collect_prefix_chunks(self.head.as_ref(), &mut chunks);
        chunks.extend(self.tail.sealed.iter().cloned());

        if let Some(active) = self.active_chunk() {
            chunks.push(active.clone());
        }

        chunks
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
            header.max_events_per_chunk = TRACE_DEFAULT_MAX_EVENTS_PER_CHUNK;
        }
        if header.max_chunk_bytes == 0 {
            header.max_chunk_bytes = TRACE_DEFAULT_MAX_CHUNK_BYTES;
        }

        let next_sequence = TraceSequence::new(0);

        Self {
            state: Arc::new(Mutex::new(TraceState {
                header,
                checkpoints: Vec::new(),
                next_sequence,
                head: None,
                tail: TraceTail::new(next_sequence),
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
        let chunks = state.chunks();

        build_trailer(&chunks, state.checkpoints.clone())
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

    /// Capture one full trace file.
    pub(super) fn file(&self) -> TraceFile {
        let mut state = self.state.lock();

        // materialize the current tail so captured images share immutable prefixes
        state.materialize_tail();

        TraceFile::new(
            state.header.clone(),
            state.chunks(),
            state.checkpoints.clone(),
        )
    }

    /// Restore one full trace file.
    pub(super) fn restore_file(&self, file: TraceFile) -> RuntimeResult<()> {
        file.validate()?;

        let mut current = self.state.lock();
        let next_sequence = file.next_sequence();
        let (header, chunks, trailer) = file.into_parts();
        let checkpoints = trailer.checkpoints;
        let head = (!chunks.is_empty()).then(|| Arc::new(TracePrefix::new(None, chunks)));

        // restore one fresh empty tail after the captured immutable history
        let tail = TraceTail::new(next_sequence);

        *current = TraceState {
            header,
            checkpoints,
            next_sequence,
            head,
            tail,
        };

        Ok(())
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
        if encoded_len > u32::MAX as u64 {
            return Err(RuntimeError::TraceEncodeFailed {
                name: "event".to_string(),
            }
            .boxed());
        }
        let record_len = encoded_len + TRACE_EVENT_LENGTH_BYTES as u64;

        let mut state = self.state.lock();
        let sequence = state.next_sequence;
        state.next_sequence = state.next_sequence.next();

        // rotate the active chunk before appending when it is full
        let is_rotation_required = !state.tail.active.is_empty()
            && state.tail.active.should_rotate_for_event(
                record_len,
                state.header.max_events_per_chunk as usize,
                state.header.max_chunk_bytes,
            );
        if is_rotation_required {
            state.seal_active_chunk(sequence);
        }

        // append the encoded event to the active chunk
        let chunk = &mut state.tail.active;
        let start = chunk.bytes.len();
        let payload_start = start + TRACE_EVENT_LENGTH_BYTES;
        let end = start + record_len as usize;
        chunk.bytes.resize(end, 0);
        chunk.bytes[start..payload_start].copy_from_slice(&(encoded_len as u32).to_le_bytes());
        let encoded_len = postcard::to_slice(&event, &mut chunk.bytes[payload_start..end])
            .map_err(|_| {
                RuntimeError::TraceEncodeFailed {
                    name: "event".to_string(),
                }
                .boxed()
            })?
            .len();
        let encoded_end = payload_start + encoded_len;

        chunk.bytes.truncate(encoded_end);
        chunk.header.event_count += 1;

        Ok(sequence)
    }

    /// Record a checkpoint index entry.
    pub fn record_checkpoint(&self, mut checkpoint: TraceCheckpointIndex) -> RuntimeResult<()> {
        // align the checkpoint with the next trace sequence
        let sequence = self.next_sequence();
        checkpoint.sequence = sequence;

        self.record_checkpoint_exact(checkpoint)
    }

    /// Record a checkpoint index entry with an explicit sequence boundary.
    pub fn record_checkpoint_exact(&self, checkpoint: TraceCheckpointIndex) -> RuntimeResult<()> {
        let sequence = checkpoint.sequence.get();

        // append the checkpoint entry in sequence order
        let mut state = self.state.lock();
        let insert_index = state
            .checkpoints
            .partition_point(|entry| entry.sequence.get() <= sequence);
        state.checkpoints.insert(insert_index, checkpoint);

        Ok(())
    }
}

/// Collect prefix chunks from oldest to newest.
fn collect_prefix_chunks(head: Option<&Arc<TracePrefix>>, chunks: &mut Vec<TraceChunk>) {
    let mut prefixes = Vec::new();
    let mut current = head.cloned();
    while let Some(prefix) = current {
        current = prefix.parent.clone();
        prefixes.push(prefix);
    }

    prefixes.reverse();
    for prefix in prefixes {
        chunks.extend(prefix.chunks.iter().cloned());
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::runtime::time::Instant;
    use crate::world::trace::{Outcome, TraceRecord};
    use crate::world::{CheckpointId, RevisionId};
    use destack_workspace::Environment;

    /// Build one explicit trace header for log tests.
    fn test_trace_header() -> TraceHeader {
        TraceHeader::new(Environment::default())
    }

    /// Capture one trace image should materialize the local tail into shared history.
    #[test]
    fn test_image_materializes_shared_history() {
        let log = TraceLog::new(test_trace_header());
        log.record_event(TraceRecord::Outcome(Outcome::TimeAdvance(Instant::new(1))))
            .expect("record tick");

        let file = log.file();
        let state = log.state.lock();

        assert_eq!(file.chunks.len(), 1);
        assert_eq!(state.head_chunk_count(), 1);
        assert!(state.tail.active.is_empty());
        assert!(state.tail.sealed.is_empty());
    }

    /// Appending after one captured trace image should keep the shared head stable.
    #[test]
    fn test_record_after_image_keeps_shared_head() {
        let log = TraceLog::new(test_trace_header());
        log.record_event(TraceRecord::Outcome(Outcome::TimeAdvance(Instant::new(1))))
            .expect("record first tick");
        log.file();
        let head = log.state.lock().head.clone().expect("state head");

        log.record_event(TraceRecord::Outcome(Outcome::TimeAdvance(Instant::new(2))))
            .expect("record second tick");

        let state = log.state.lock();
        assert!(Arc::ptr_eq(state.head.as_ref().expect("state head"), &head));
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
            revision_id: RevisionId::new(2),
            sequence: TraceSequence::new(5),
            path: "memory://checkpoint/2".to_string(),
            hash: 22,
            size_bytes: 2,
        })
        .expect("record later checkpoint");
        log.record_checkpoint_exact(TraceCheckpointIndex {
            checkpoint_id: CheckpointId::new(1),
            revision_id: RevisionId::new(1),
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
        let file = log.file();

        log.record_event(TraceRecord::Outcome(Outcome::TimeAdvance(Instant::new(2))))
            .expect("record second tick");
        log.restore_file(file).expect("restore file");
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

    /// Appending after restore should keep cursor validation consistent.
    #[test]
    fn test_record_after_restore_keeps_cursor_validation_consistent() {
        let log = TraceLog::new(test_trace_header());
        log.record_event(TraceRecord::Outcome(Outcome::TimeAdvance(Instant::new(1))))
            .expect("record first tick");
        let file = log.file();

        log.restore_file(file).expect("restore file");
        log.record_event(TraceRecord::Outcome(Outcome::TimeAdvance(Instant::new(2))))
            .expect("record second tick");
        let mut cursor = log.reader();

        let first = cursor.next_event().expect("read first event");
        match first {
            Some(TraceRecord::Outcome(Outcome::TimeAdvance(deadline))) => {
                assert_eq!(deadline, Instant::new(1));
            }
            other => panic!("unexpected first event: {other:?}"),
        }

        let second = cursor.next_event().expect("read second event");
        match second {
            Some(TraceRecord::Outcome(Outcome::TimeAdvance(deadline))) => {
                assert_eq!(deadline, Instant::new(2));
            }
            other => panic!("unexpected second event: {other:?}"),
        }
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
