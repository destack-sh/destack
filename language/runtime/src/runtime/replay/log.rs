use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::runtime::replay::{
    TraceCheckpointIndex, TraceCursor, TraceEvent, TraceHeader, TraceSegmentIndex, TraceTrailer,
};
use crate::runtime::world::BranchId;
use destack_core::{FNV_OFFSET_BASIS_128, fnv1a_128_update};
use postcard::experimental::serialized_size;

use super::chunk::TraceSegment;

/// Default maximum number of events in a chunk.
const DEFAULT_MAX_EVENTS_PER_CHUNK: usize = 1024;
/// Default maximum chunk size in bytes.
const DEFAULT_MAX_CHUNK_BYTES: u64 = 4 * 1024 * 1024;

/// Sequence number for events within a trace log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Materialized trace-log image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TraceLogImage {
    /// Trace log header metadata.
    header: TraceHeader,
    /// Trace log trailer metadata.
    trailer: TraceTrailer,
    /// Maximum number of events per segment.
    max_events_per_chunk: usize,
    /// Maximum segment size in bytes.
    max_chunk_bytes: u64,
    /// Next sequence number to assign.
    next_sequence: TraceSequence,
    /// Next segment offset for trailer entries.
    next_offset: u64,
    /// Shared immutable prefix segments for this branch.
    pub(super) prefix_segments: Arc<Vec<Arc<TraceSegment>>>,
    /// Branch-local tail segments for this branch.
    pub(super) tail_segments: Vec<Arc<TraceSegment>>,
}

impl TraceLogImage {
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

    /// Return the total number of stored segments.
    pub(super) fn segment_count(&self) -> usize {
        self.prefix_segments.len() + self.tail_segments.len()
    }

    /// Return one stored segment by stable index.
    pub(super) fn segment(&self, index: usize) -> Option<&Arc<TraceSegment>> {
        if index < self.prefix_segments.len() {
            return self.prefix_segments.get(index);
        }

        self.tail_segments.get(index - self.prefix_segments.len())
    }
}

/// Trace log for deterministic execution.
#[derive(Debug, Clone)]
pub struct TraceLog {
    // NOTE #Incomplete: persist chunks to disk and stream across threads
    /// Shared trace log state.
    state: Arc<Mutex<TraceLogImage>>,
}

impl Default for TraceLog {
    fn default() -> Self {
        Self::new(TraceHeader::default())
    }
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

        // seed the first segment and trailer entry
        let segment = Arc::new(TraceSegment::new(0, TraceSequence::new(0)));
        let trailer = TraceTrailer {
            segments: vec![TraceSegmentIndex {
                index: 0,
                offset: 0,
                length: 0,
                checksum: 0,
            }],
            checkpoints: Vec::new(),
            log_hash: 0,
        };

        Self {
            state: Arc::new(Mutex::new(TraceLogImage {
                header,
                trailer,
                max_events_per_chunk,
                max_chunk_bytes,
                next_sequence: TraceSequence::new(0),
                next_offset: 0,
                prefix_segments: Arc::new(Vec::new()),
                tail_segments: vec![segment],
            })),
        }
    }

    /// Return the trace log header.
    pub fn header(&self) -> TraceHeader {
        // lock state for reading
        let state = self.state.lock();
        state.header.clone()
    }

    /// Return the trace log trailer.
    pub fn trailer(&self) -> TraceTrailer {
        // lock state for reading
        let state = self.state.lock();
        let mut trailer = state.trailer.clone();
        trailer.log_hash = compute_log_hash(&trailer.segments, &trailer.checkpoints);
        trailer
    }

    /// Return the current branch identifier.
    pub fn branch_id(&self) -> BranchId {
        // lock state for reading
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
        // lock state for reading
        let state = self.state.lock();
        state.next_sequence
    }

    /// Capture one full trace-log image.
    pub(crate) fn image(&self) -> TraceLogImage {
        let state = self.state.lock();
        let mut image = state.clone();
        let mut prefix_segments = Vec::with_capacity(state.segment_count());

        prefix_segments.extend(state.prefix_segments.iter().cloned());
        prefix_segments.extend(state.tail_segments.iter().cloned());
        image.prefix_segments = Arc::new(prefix_segments);
        image.tail_segments.clear();

        image
    }

    /// Restore one full trace-log image.
    pub(crate) fn restore_image(&self, image: TraceLogImage) {
        let mut current = self.state.lock();

        *current = image;
    }

    /// Record an event in the log.
    pub(crate) fn record_event(&self, event: TraceEvent) -> RuntimeResult<TraceSequence> {
        // compute the encoded size ahead of time
        let encoded_len = serialized_size(&event).map_err(|_| {
            RuntimeError::TraceEncodeFailed {
                name: "event".to_string(),
            }
            .boxed()
        })? as u64;

        // lock state for mutation
        let mut state = self.state.lock();

        // assign the next sequence
        let sequence = state.next_sequence;
        state.next_sequence = state.next_sequence.next();

        // append the event payload
        let is_rotation_required = state.tail_segments.last().is_none_or(|segment| {
            segment.should_rotate_for_event(
                encoded_len,
                state.max_events_per_chunk,
                state.max_chunk_bytes,
            )
        });
        if is_rotation_required {
            // finalize the current segment before creating a new one
            finalize_segment(&mut state);

            // create the next segment and trailer entry
            let next_index = state.segment_count() as u32;
            let offset = state.next_offset;
            let segment = Arc::new(TraceSegment::new(next_index, sequence));
            state.tail_segments.push(segment);
            state.trailer.segments.push(TraceSegmentIndex {
                index: next_index,
                offset,
                length: 0,
                checksum: 0,
            });
        }

        // append the event to the active segment
        let segment = state
            .tail_segments
            .last_mut()
            .expect("trace log must have an active segment");
        let segment = Arc::make_mut(segment);
        let start = segment.data.len();
        let end = start + encoded_len as usize;
        segment.data.resize(end, 0);
        let encoded_len = postcard::to_slice(&event, &mut segment.data[start..end])
            .map_err(|_| {
                RuntimeError::TraceEncodeFailed {
                    name: "event".to_string(),
                }
                .boxed()
            })?
            .len();
        let encoded_end = start + encoded_len;

        // checksum only the encoded event bytes
        segment.update_checksum_for_range(start, encoded_end);

        // trim trailing capacity when serialized_size overestimates
        segment.data.truncate(encoded_end);
        segment.header.byte_length = segment.data.len() as u64;
        segment.event_lengths.push(encoded_len as u32);
        segment.header.event_count = segment.event_lengths.len() as u32;
        segment.header.sequence_end = sequence;

        // keep the trailer entry in sync
        update_trailer_entry(&mut state);

        Ok(sequence)
    }

    /// Record a checkpoint index entry in trailer metadata.
    pub fn record_checkpoint(&self, mut checkpoint: TraceCheckpointIndex) -> RuntimeResult<()> {
        // align the checkpoint with the next log sequence
        let sequence = self.next_sequence();
        checkpoint.sequence = sequence;

        // update trailer index
        let mut state = self.state.lock();
        state.trailer.checkpoints.push(checkpoint);

        Ok(())
    }
}

/// Finalize the active segment trailer metadata before rotating.
fn finalize_segment(state: &mut TraceLogImage) {
    // capture metadata for the trailing segment entry
    let Some(segment) = state.tail_segments.last() else {
        return;
    };
    let segment = segment.as_ref();
    if let Some(entry) = state.trailer.segments.last_mut() {
        entry.length = segment.header.byte_length;
        entry.checksum = segment.header.checksum;
    }

    // advance the next segment offset
    state.next_offset = state.next_offset.saturating_add(segment.header.byte_length);
}

/// Refresh the trailing segment trailer entry after one append.
fn update_trailer_entry(state: &mut TraceLogImage) {
    // update the trailing segment index entry
    let Some(segment) = state.tail_segments.last() else {
        return;
    };
    let segment = segment.as_ref();
    if let Some(entry) = state.trailer.segments.last_mut() {
        entry.length = segment.header.byte_length;
        entry.checksum = segment.header.checksum;
    }
}

pub(super) fn compute_log_hash(
    segments: &[TraceSegmentIndex],
    checkpoints: &[TraceCheckpointIndex],
) -> u128 {
    // hash the segment index metadata
    let mut hash = FNV_OFFSET_BASIS_128;
    for segment in segments {
        hash = fnv1a_128_update(hash, &segment.index.to_le_bytes());
        hash = fnv1a_128_update(hash, &segment.offset.to_le_bytes());
        hash = fnv1a_128_update(hash, &segment.length.to_le_bytes());
        hash = fnv1a_128_update(hash, &segment.checksum.to_le_bytes());
    }

    // hash the checkpoint metadata
    for checkpoint in checkpoints {
        hash = fnv1a_128_update(hash, &checkpoint.checkpoint_id.get().to_le_bytes());
        hash = fnv1a_128_update(hash, &checkpoint.revision_id.get().to_le_bytes());
        hash = fnv1a_128_update(hash, &checkpoint.sequence.get().to_le_bytes());
        hash = fnv1a_128_update(hash, &checkpoint.size_bytes.to_le_bytes());
        hash = fnv1a_128_update(hash, &checkpoint.hash.to_le_bytes());
        hash = fnv1a_128_update(hash, checkpoint.path.as_bytes());
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::replay::TraceEvent;
    use crate::runtime::time::WorldInstant;

    /// Capturing the trace-log image should share immutable segment backing.
    #[test]
    fn test_capture_store_shares_segment_backing() {
        // record one event so the first segment has payload
        let log = TraceLog::new(TraceHeader::default());
        log.record_event(TraceEvent::Tick(WorldInstant::new(1)))
            .expect("record tick");

        // capture one immutable snapshot of the log state
        let snapshot = log.image();

        // verify the captured store shares the current segment backing
        let state = log.state.lock();
        assert_eq!(state.prefix_segments.len(), 0);
        assert_eq!(state.tail_segments.len(), 1);
        assert_eq!(snapshot.prefix_segments.len(), 1);
        assert_eq!(snapshot.tail_segments.len(), 0);
        assert!(Arc::ptr_eq(
            &state.tail_segments[0],
            &snapshot.prefix_segments[0]
        ));
    }

    /// Appending after one captured log image should detach only the active segment.
    #[test]
    fn test_record_after_snapshot_detaches_active_segment() {
        // record one event and capture one shared snapshot
        let log = TraceLog::new(TraceHeader::default());
        log.record_event(TraceEvent::Tick(WorldInstant::new(1)))
            .expect("record first tick");
        let snapshot = log.image();

        // append one more event to the live log
        log.record_event(TraceEvent::Tick(WorldInstant::new(2)))
            .expect("record second tick");

        // verify the live log detached its active segment from the captured store
        let state = log.state.lock();
        assert_eq!(state.prefix_segments.len(), 0);
        assert_eq!(state.tail_segments.len(), 1);
        assert_eq!(snapshot.prefix_segments.len(), 1);
        assert_eq!(snapshot.tail_segments.len(), 0);
        assert!(!Arc::ptr_eq(
            &state.tail_segments[0],
            &snapshot.prefix_segments[0]
        ));

        // verify the captured store still sees only the original event bytes
        assert_eq!(snapshot.prefix_segments[0].header.event_count, 1);
        assert_eq!(state.tail_segments[0].header.event_count, 2);
    }

    /// Restoring one captured log image should keep existing cursors usable.
    #[test]
    fn test_restore_store_keeps_cursor_validation_consistent() {
        // record one event and capture both the store and the cursor
        let log = TraceLog::new(TraceHeader::default());
        log.record_event(TraceEvent::Tick(WorldInstant::new(1)))
            .expect("record first tick");
        let cursor = log.reader();
        let cursor_image = cursor.capture_image();
        let store_image = log.image();

        // mutate the live log and then restore the captured store and cursor
        log.record_event(TraceEvent::Tick(WorldInstant::new(2)))
            .expect("record second tick");
        log.restore_image(store_image);
        cursor.restore_image(cursor_image).expect("restore cursor");

        // the restored cursor should read the restored store without hash mismatch
        let event = cursor.next_event().expect("read restored event");
        match event {
            Some(TraceEvent::Tick(deadline)) => {
                assert_eq!(deadline, WorldInstant::new(1));
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
        let log = TraceLog::new(TraceHeader::default());
        log.record_event(TraceEvent::Tick(WorldInstant::new(1)))
            .expect("record first tick");
        log.record_event(TraceEvent::Tick(WorldInstant::new(2)))
            .expect("record second tick");

        let cursor = log.reader();
        cursor
            .seek_sequence(TraceSequence::new(1))
            .expect("seek cursor");

        let event = cursor.next_event().expect("read sought event");
        match event {
            Some(TraceEvent::Tick(deadline)) => {
                assert_eq!(deadline, WorldInstant::new(2));
            }
            other => panic!("unexpected sought event: {other:?}"),
        }
        assert_eq!(cursor.tell(), TraceSequence::new(2));
    }
}
